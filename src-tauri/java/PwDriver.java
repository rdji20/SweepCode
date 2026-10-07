import java.io.*;
import java.lang.reflect.*;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.Permission;
import java.util.*;

/**
 * Runs a LeetCode-style solution against test inputs.
 *
 * Usage: java PwDriver <job-file> <result-file>
 *
 * Job file: "key=value" header lines, a line "---", then the raw input lines.
 *   mode=function  method=<name> params=<n> outputParam=<i or -1>
 *   mode=design    class=<ClassName>
 * Function mode reads <n> lines per test, design mode reads 2 lines per test.
 *
 * Results go to <result-file> as JSON lines, flushed after every line, so the
 * host can tell which test was running if the process gets killed.
 */
public final class PwDriver {
    private static final int STDOUT_CAP = 64 * 1024;
    private static final int MAX_NODES = 100_000;
    private static PrintStream results;
    private static volatile boolean guardArmed = false;

    public static void main(String[] args) throws Exception {
        results = new PrintStream(new FileOutputStream(args[1]), true, "UTF-8");
        List<String> lines = Files.readAllLines(Paths.get(args[0]), StandardCharsets.UTF_8);
        Map<String, String> header = new HashMap<>();
        int k = 0;
        for (; k < lines.size(); k++) {
            String l = lines.get(k);
            if (l.equals("---")) { k++; break; }
            int eq = l.indexOf('=');
            if (eq > 0) header.put(l.substring(0, eq), l.substring(eq + 1));
        }
        List<String> inputs = new ArrayList<>(lines.subList(Math.min(k, lines.size()), lines.size()));
        while (!inputs.isEmpty() && inputs.get(inputs.size() - 1).trim().isEmpty()) inputs.remove(inputs.size() - 1);

        boolean sm = installGuard();
        emit("{\"t\":\"meta\",\"guard\":" + sm + ",\"java\":" + PwJson.quote(System.getProperty("java.version")) + "}");

        final PrintStream realOut = System.out;
        final PrintStream realErr = System.err;
        try {
            if ("design".equals(header.get("mode"))) runDesign(header, inputs);
            else runFunction(header, inputs);
        } catch (Fatal f) {
            emit("{\"t\":\"fatal\",\"error\":" + PwJson.quote(f.getMessage()) + "}");
        } finally {
            System.setOut(realOut);
            System.setErr(realErr);
        }
        emit("{\"t\":\"end\"}");
        guardArmed = false;
        results.flush();
        System.exit(0);
    }

    // ---------------------------------------------------------------- guard

    /** Blocks System.exit, process launching, file writes and sockets from user code. */
    private static boolean installGuard() {
        try {
            System.setSecurityManager(new SecurityManager() {
                @Override public void checkPermission(Permission p) {
                    if (!guardArmed) return;
                    String name = p.getName();
                    if (p instanceof RuntimePermission) {
                        if (name.startsWith("exitVM")) throw new SecurityException("System.exit() is blocked here; return from your method instead");
                        if (name.equals("setSecurityManager")) throw new SecurityException("Changing the security manager is blocked");
                    } else if (p instanceof FilePermission) {
                        String a = p.getActions();
                        if (a.contains("execute")) throw new SecurityException("Running programs is blocked: " + name);
                        if (a.contains("write") || a.contains("delete")) throw new SecurityException("Writing files is blocked: " + name);
                    } else if (p instanceof java.net.SocketPermission || p instanceof java.net.NetPermission) {
                        throw new SecurityException("Network access is blocked: " + name);
                    }
                }
                @Override public void checkPermission(Permission p, Object context) { checkPermission(p); }
            });
            return true;
        } catch (Throwable t) {
            return false; // JDK 24+ removed the SecurityManager; the OS sandbox still applies.
        }
    }

    // ------------------------------------------------------------- function

    private static void runFunction(Map<String, String> h, List<String> inputs) throws Exception {
        String methodName = h.get("method");
        int params = Integer.parseInt(h.getOrDefault("params", "0"));
        int outputParam = Integer.parseInt(h.getOrDefault("outputParam", "-1"));
        Class<?> cls = loadClass("Solution");
        Method m = findMethod(cls, methodName, params);
        Constructor<?> ctor;
        try {
            ctor = cls.getDeclaredConstructor();
            ctor.setAccessible(true);
        } catch (NoSuchMethodException e) {
            throw new Fatal("class Solution needs a no-argument constructor");
        }
        Type[] types = m.getGenericParameterTypes();
        String[] paramNames = h.getOrDefault("paramNames", "").split(",");
        int perTest = Math.max(params, 1);
        if (params == 0) perTest = 1;
        int tests = params == 0 ? Math.max(1, inputs.size()) : inputs.size() / params;
        if (params > 0 && inputs.size() % params != 0) {
            throw new Fatal("each test needs " + params + " input lines but there are " + inputs.size() + " lines in total");
        }
        for (int t = 0; t < tests; t++) {
            emit("{\"t\":\"start\",\"i\":" + t + "}");
            Object[] args = new Object[params];
            try {
                for (int p = 0; p < params; p++) {
                    String raw = inputs.get(t * perTest + p);
                    String label = p < paramNames.length && !paramNames[p].isEmpty() ? paramNames[p] : "parameter " + (p + 1);
                    try {
                        args[p] = convert(PwJson.parse(raw), types[p]);
                    } catch (RuntimeException e) {
                        throw new InputError(label + ": " + e.getMessage());
                    }
                }
            } catch (InputError e) {
                emitError(t, "input", e.getMessage(), null, "", false, 0);
                continue;
            }
            final Object[] callArgs = args;
            final Class<?> outType = m.getReturnType() == void.class
                ? (outputParam >= 0 && outputParam < params ? m.getParameterTypes()[outputParam] : void.class)
                : m.getReturnType();
            Outcome o = runIsolated(() -> {
                Object inst = ctor.newInstance();
                Object ret = m.invoke(inst, callArgs);
                if (m.getReturnType() == void.class) {
                    return outputParam >= 0 && outputParam < callArgs.length ? callArgs[outputParam] : null;
                }
                return ret;
            });
            report(t, o, m.getReturnType() == void.class && outputParam < 0, outType);
        }
    }

    // --------------------------------------------------------------- design

    private static void runDesign(Map<String, String> h, List<String> inputs) throws Exception {
        String className = h.get("class");
        Class<?> cls = loadClass(className);
        if (inputs.size() % 2 != 0) throw new Fatal("design tests need 2 lines each (method names, then arguments)");
        for (int t = 0; t < inputs.size() / 2; t++) {
            emit("{\"t\":\"start\",\"i\":" + t + "}");
            List<?> names, argLists;
            try {
                names = asList(PwJson.parse(inputs.get(2 * t)), "method names line");
                argLists = asList(PwJson.parse(inputs.get(2 * t + 1)), "arguments line");
                if (names.size() != argLists.size()) throw new IllegalArgumentException(names.size() + " calls but " + argLists.size() + " argument lists");
                if (names.isEmpty() || !className.equals(names.get(0))) throw new IllegalArgumentException("the first call must be " + className);
            } catch (RuntimeException e) {
                emitError(t, "input", e.getMessage(), null, "", false, 0);
                continue;
            }
            final List<?> fNames = names, fArgs = argLists;
            Outcome o = runIsolated(() -> {
                List<Object> out = new ArrayList<>();
                Object inst = null;
                for (int c = 0; c < fNames.size(); c++) {
                    String name = String.valueOf(fNames.get(c));
                    List<?> raw = asList(fArgs.get(c), "arguments of call " + (c + 1));
                    if (c == 0) {
                        Constructor<?> ctor = findCtor(cls, raw.size());
                        inst = ctor.newInstance(convertAll(raw, ctor.getGenericParameterTypes(), name));
                        out.add(null);
                    } else {
                        Method m = findMethod(cls, name, raw.size());
                        Object r = m.invoke(inst, convertAll(raw, m.getGenericParameterTypes(), name));
                        out.add(m.getReturnType() == void.class ? null : r);
                    }
                }
                return out;
            });
            report(t, o, false, Object.class);
        }
    }

    private static Object[] convertAll(List<?> raw, Type[] types, String call) {
        Object[] a = new Object[raw.size()];
        for (int i = 0; i < a.length; i++) {
            try {
                a[i] = convert(raw.get(i), types[i]);
            } catch (RuntimeException e) {
                throw new InputError("argument " + (i + 1) + " of " + call + ": " + e.getMessage());
            }
        }
        return a;
    }

    // ------------------------------------------------------------ execution

    private interface Body { Object call() throws Throwable; }

    private static final class Outcome {
        Object value;
        Throwable error;
        String stdout = "";
        boolean stdoutTruncated;
        double ms;
    }

    /** Runs one test in a thread with a big stack and captured stdout/stderr. */
    private static Outcome runIsolated(Body body) throws InterruptedException {
        Outcome o = new Outcome();
        CappedStream cap = new CappedStream(STDOUT_CAP);
        PrintStream ps;
        try {
            ps = new PrintStream(cap, true, "UTF-8");
        } catch (UnsupportedEncodingException e) {
            throw new AssertionError(e);
        }
        PrintStream realOut = System.out, realErr = System.err;
        Thread th = new Thread(null, () -> {
            long start = System.nanoTime();
            try {
                o.value = body.call();
            } catch (Throwable t) {
                o.error = t;
            } finally {
                o.ms = (System.nanoTime() - start) / 1e6;
            }
        }, "solution", 512L << 20);
        System.setOut(ps);
        System.setErr(ps);
        guardArmed = true;
        th.start();
        th.join();
        guardArmed = false;
        System.setOut(realOut);
        System.setErr(realErr);
        ps.flush();
        o.stdout = cap.text();
        o.stdoutTruncated = cap.truncated;
        return o;
    }

    private static void report(int t, Outcome o, boolean noOutput, Class<?> outType) {
        if (o.error == null) {
            String out;
            try {
                if (noOutput) out = "null";
                else if (o.value == null && (outType == ListNode.class || outType == TreeNode.class)) out = "[]";
                else out = serialize(o.value);
            } catch (Throwable e) {
                emitError(t, "exception", "could not print the return value: " + e, null, o.stdout, o.stdoutTruncated, o.ms);
                return;
            }
            emit("{\"t\":\"done\",\"i\":" + t + ",\"status\":\"ok\",\"output\":" + PwJson.quote(out)
                + ",\"stdout\":" + PwJson.quote(o.stdout) + ",\"stdoutTruncated\":" + o.stdoutTruncated
                + ",\"ms\":" + String.format(Locale.ROOT, "%.3f", o.ms) + "}");
            return;
        }
        Throwable e = o.error;
        while ((e instanceof InvocationTargetException || e instanceof UndeclaredThrowableException) && e.getCause() != null) {
            e = e.getCause();
        }
        String kind = "exception";
        if (e instanceof OutOfMemoryError) kind = "memory";
        else if (e instanceof StackOverflowError) kind = "stackoverflow";
        else if (e instanceof SecurityException) kind = "blocked";
        else if (e instanceof InputError) kind = "input";
        String msg = e instanceof InputError ? e.getMessage() : e.toString();
        emitError(t, kind, msg, userFrames(e), o.stdout, o.stdoutTruncated, o.ms);
    }

    /** Stack frames from the user's code only (skips JDK internals and the driver). */
    private static List<String> userFrames(Throwable e) {
        List<String> out = new ArrayList<>();
        StackTraceElement[] st = e.getStackTrace();
        for (StackTraceElement f : st) {
            String c = f.getClassName();
            if (c.startsWith("java.") || c.startsWith("jdk.") || c.startsWith("sun.") || c.startsWith("PwDriver")) continue;
            out.add(f.toString());
            if (out.size() >= 30) break;
        }
        if (e instanceof StackOverflowError && out.size() > 6) {
            List<String> shortList = new ArrayList<>(out.subList(0, 6));
            shortList.add("... (" + st.length + " frames, recursion too deep)");
            return shortList;
        }
        return out;
    }

    private static void emitError(int t, String kind, String msg, List<String> trace, String stdout, boolean trunc, double ms) {
        StringBuilder b = new StringBuilder();
        b.append("{\"t\":\"done\",\"i\":").append(t).append(",\"status\":\"error\",\"errorKind\":").append(PwJson.quote(kind))
            .append(",\"error\":").append(PwJson.quote(msg == null ? "" : msg)).append(",\"trace\":[");
        if (trace != null) {
            for (int i = 0; i < trace.size(); i++) {
                if (i > 0) b.append(',');
                b.append(PwJson.quote(trace.get(i)));
            }
        }
        b.append("],\"stdout\":").append(PwJson.quote(stdout)).append(",\"stdoutTruncated\":").append(trunc)
            .append(",\"ms\":").append(String.format(Locale.ROOT, "%.3f", ms)).append('}');
        emit(b.toString());
    }

    private static synchronized void emit(String line) {
        results.println(line);
        results.flush();
    }

    // ------------------------------------------------------------ reflection

    private static Class<?> loadClass(String name) {
        try {
            return Class.forName(name);
        } catch (ClassNotFoundException e) {
            throw new Fatal("class " + name + " was not found. Keep the class name LeetCode gave you.");
        }
    }

    private static Method findMethod(Class<?> cls, String name, int params) {
        Method best = null;
        for (Method m : cls.getDeclaredMethods()) {
            if (!m.getName().equals(name)) continue;
            if (m.getParameterCount() == params) { best = m; break; }
            if (best == null) best = m;
        }
        if (best == null) throw new Fatal("method " + name + " was not found in class " + cls.getName() + ". Keep the method name LeetCode gave you.");
        if (best.getParameterCount() != params) {
            throw new Fatal("method " + name + " takes " + best.getParameterCount() + " parameters but the test gives " + params);
        }
        best.setAccessible(true);
        return best;
    }

    private static Constructor<?> findCtor(Class<?> cls, int params) {
        for (Constructor<?> c : cls.getDeclaredConstructors()) {
            if (c.getParameterCount() == params) { c.setAccessible(true); return c; }
        }
        throw new Fatal("class " + cls.getName() + " has no constructor with " + params + " parameters");
    }

    // ------------------------------------------------------------ conversion

    private static List<?> asList(Object v, String what) {
        if (!(v instanceof List)) throw new IllegalArgumentException(what + " must be an array, got " + describe(v));
        return (List<?>) v;
    }

    private static String describe(Object v) {
        if (v == null) return "null";
        if (v instanceof String) return "string " + PwJson.quote((String) v);
        if (v instanceof List) return "array";
        return v.getClass().getSimpleName().toLowerCase(Locale.ROOT) + " " + v;
    }

    private static long toLong(Object v, String type) {
        if (v instanceof Long) return (Long) v;
        if (v instanceof BigInteger) throw new IllegalArgumentException(v + " is too large for " + type);
        if (v instanceof Double && ((Double) v) == Math.rint((Double) v)) return ((Double) v).longValue();
        throw new IllegalArgumentException("expected " + type + ", got " + describe(v));
    }

    private static int toInt(Object v) {
        long l = toLong(v, "int");
        if (l < Integer.MIN_VALUE || l > Integer.MAX_VALUE) throw new IllegalArgumentException(l + " does not fit in an int");
        return (int) l;
    }

    private static double toDouble(Object v) {
        if (v instanceof Number) return ((Number) v).doubleValue();
        throw new IllegalArgumentException("expected a number, got " + describe(v));
    }

    static Object convert(Object v, Type t) {
        if (t instanceof Class) {
            Class<?> c = (Class<?>) t;
            if (c == int.class || c == Integer.class) { if (v == null && c == Integer.class) return null; return toInt(v); }
            if (c == long.class || c == Long.class) { if (v == null && c == Long.class) return null; return toLong(v, "long"); }
            if (c == short.class || c == Short.class) return (short) toInt(v);
            if (c == byte.class || c == Byte.class) return (byte) toInt(v);
            if (c == double.class || c == Double.class) { if (v == null && c == Double.class) return null; return toDouble(v); }
            if (c == float.class || c == Float.class) return (float) toDouble(v);
            if (c == boolean.class || c == Boolean.class) {
                if (v instanceof Boolean) return v;
                throw new IllegalArgumentException("expected true/false, got " + describe(v));
            }
            if (c == char.class || c == Character.class) {
                if (v instanceof String && ((String) v).length() == 1) return ((String) v).charAt(0);
                throw new IllegalArgumentException("expected a one-letter string like \"a\", got " + describe(v));
            }
            if (c == String.class) {
                if (v == null) return null;
                if (v instanceof String) return v;
                throw new IllegalArgumentException("expected a string in double quotes, got " + describe(v));
            }
            if (c == ListNode.class) return toListNode(v);
            if (c == TreeNode.class) return toTree(v);
            if (c.isArray()) {
                if (v == null) return null;
                List<?> l = asList(v, "value");
                Object arr = Array.newInstance(c.getComponentType(), l.size());
                for (int i = 0; i < l.size(); i++) Array.set(arr, i, convert(l.get(i), c.getComponentType()));
                return arr;
            }
            if (c == Object.class) return natural(v);
            if (List.class.isAssignableFrom(c) || c == Collection.class || c == Iterable.class) return natural(v);
            throw new IllegalArgumentException("parameter type " + c.getSimpleName() + " is not supported yet");
        }
        if (t instanceof ParameterizedType) {
            ParameterizedType pt = (ParameterizedType) t;
            Class<?> raw = (Class<?>) pt.getRawType();
            Type arg = pt.getActualTypeArguments()[0];
            if (raw == List.class || raw == ArrayList.class || raw == Collection.class || raw == Iterable.class
                || raw == LinkedList.class || raw == Deque.class || raw == Queue.class) {
                if (v == null) return null;
                List<?> l = asList(v, "value");
                List<Object> out = (raw == LinkedList.class || raw == Deque.class || raw == Queue.class) ? new LinkedList<>() : new ArrayList<>();
                for (Object e : l) out.add(convert(e, arg));
                return out;
            }
            if (raw == Set.class || raw == HashSet.class) {
                Set<Object> out = new HashSet<>();
                for (Object e : asList(v, "value")) out.add(convert(e, arg));
                return out;
            }
            throw new IllegalArgumentException("parameter type " + t.getTypeName() + " is not supported yet");
        }
        if (t instanceof GenericArrayType) {
            Type comp = ((GenericArrayType) t).getGenericComponentType();
            List<?> l = asList(v, "value");
            Class<?> rawComp = comp instanceof ParameterizedType ? (Class<?>) ((ParameterizedType) comp).getRawType() : Object.class;
            Object arr = Array.newInstance(rawComp, l.size());
            for (int i = 0; i < l.size(); i++) Array.set(arr, i, convert(l.get(i), comp));
            return arr;
        }
        throw new IllegalArgumentException("parameter type " + t.getTypeName() + " is not supported yet");
    }

    /** JSON value to plain Java: ints become Integer when they fit. */
    private static Object natural(Object v) {
        if (v instanceof Long) {
            long l = (Long) v;
            return (l >= Integer.MIN_VALUE && l <= Integer.MAX_VALUE) ? (Object) (int) l : (Object) l;
        }
        if (v instanceof List) {
            List<Object> out = new ArrayList<>();
            for (Object e : (List<?>) v) out.add(natural(e));
            return out;
        }
        return v;
    }

    private static ListNode toListNode(Object v) {
        if (v == null) return null;
        List<?> l = asList(v, "linked list");
        ListNode dummy = new ListNode(0), cur = dummy;
        for (Object e : l) { cur.next = new ListNode(toInt(e)); cur = cur.next; }
        return dummy.next;
    }

    private static TreeNode toTree(Object v) {
        if (v == null) return null;
        List<?> l = asList(v, "tree");
        if (l.isEmpty() || l.get(0) == null) return null;
        TreeNode root = new TreeNode(toInt(l.get(0)));
        ArrayDeque<TreeNode> q = new ArrayDeque<>();
        q.add(root);
        int i = 1;
        while (!q.isEmpty() && i < l.size()) {
            TreeNode n = q.poll();
            if (i < l.size()) { Object x = l.get(i++); if (x != null) { n.left = new TreeNode(toInt(x)); q.add(n.left); } }
            if (i < l.size()) { Object x = l.get(i++); if (x != null) { n.right = new TreeNode(toInt(x)); q.add(n.right); } }
        }
        return root;
    }

    // ---------------------------------------------------------- serializing

    static String serialize(Object v) {
        StringBuilder b = new StringBuilder();
        ser(v, b);
        return b.toString();
    }

    private static void ser(Object v, StringBuilder b) {
        if (v == null) { b.append("null"); return; }
        if (v instanceof String) { b.append(PwJson.quote((String) v)); return; }
        if (v instanceof Character) { b.append(PwJson.quote(String.valueOf(v))); return; }
        if (v instanceof Double || v instanceof Float) {
            double d = ((Number) v).doubleValue();
            if (Double.isNaN(d) || Double.isInfinite(d)) b.append(d);
            else b.append(String.format(Locale.ROOT, "%.5f", d));
            return;
        }
        if (v instanceof Number || v instanceof Boolean) { b.append(v); return; }
        if (v instanceof ListNode) {
            b.append('[');
            ListNode n = (ListNode) v;
            int count = 0;
            while (n != null) {
                if (count > 0) b.append(',');
                if (++count > MAX_NODES) { b.append("...cycle?"); break; }
                b.append(n.val);
                n = n.next;
            }
            b.append(']');
            return;
        }
        if (v instanceof TreeNode) { serTree((TreeNode) v, b); return; }
        if (v.getClass().isArray()) {
            b.append('[');
            int n = Array.getLength(v);
            for (int i = 0; i < n; i++) { if (i > 0) b.append(','); ser(Array.get(v, i), b); }
            b.append(']');
            return;
        }
        if (v instanceof Iterable) {
            b.append('[');
            boolean first = true;
            for (Object e : (Iterable<?>) v) { if (!first) b.append(','); first = false; ser(e, b); }
            b.append(']');
            return;
        }
        b.append(PwJson.quote(String.valueOf(v)));
    }

    private static void serTree(TreeNode root, StringBuilder b) {
        List<String> out = new ArrayList<>();
        List<TreeNode> level = new ArrayList<>();
        level.add(root);
        int seen = 0;
        // Level order with nulls, LeetCode style; uses a list because ArrayDeque rejects null.
        int idx = 0;
        while (idx < level.size()) {
            TreeNode n = level.get(idx++);
            if (n == null) { out.add("null"); continue; }
            if (++seen > MAX_NODES) { out.add("...cycle?"); break; }
            out.add(String.valueOf(n.val));
            level.add(n.left);
            level.add(n.right);
        }
        int end = out.size();
        while (end > 0 && out.get(end - 1).equals("null")) end--;
        b.append('[').append(String.join(",", out.subList(0, end))).append(']');
    }

    // ---------------------------------------------------------------- misc

    private static final class Fatal extends RuntimeException {
        Fatal(String m) { super(m); }
    }

    private static final class InputError extends RuntimeException {
        InputError(String m) { super(m); }
    }

    /** Keeps the first `cap` bytes, silently drops the rest. */
    private static final class CappedStream extends OutputStream {
        private final ByteArrayOutputStream buf = new ByteArrayOutputStream();
        private final int cap;
        boolean truncated;
        CappedStream(int cap) { this.cap = cap; }
        @Override public synchronized void write(int b) {
            if (buf.size() < cap) buf.write(b); else truncated = true;
        }
        @Override public synchronized void write(byte[] b, int off, int len) {
            int room = cap - buf.size();
            if (len > room) { truncated = true; len = Math.max(0, room); }
            buf.write(b, off, len);
        }
        String text() { return new String(buf.toByteArray(), StandardCharsets.UTF_8); }
    }
}
