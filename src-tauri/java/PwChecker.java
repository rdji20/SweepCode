import javax.tools.*;
import java.io.*;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.util.*;

/**
 * Long-running compiler process. Keeping the JVM warm makes a check take tens
 * of milliseconds instead of a full javac startup.
 *
 * Protocol on stdin, one request at a time:
 *   PING\n                                   -> {"pong":true}
 *   COMPILE\t<fileName>\t<outDir|->\t<nChars>\n  followed by <nChars> chars (UTF-16 units) of source
 * Reply: one JSON line on stdout.
 * With outDir "-" class files go to a scratch folder and are thrown away.
 *
 * Usage: java PwChecker <supportClassesDir> <scratchDir>
 */
public final class PwChecker {
    public static void main(String[] args) throws Exception {
        String supportCp = args[0];
        String scratch = args[1];
        JavaCompiler compiler = ToolProvider.getSystemJavaCompiler();
        PrintStream out = new PrintStream(new FileOutputStream(FileDescriptor.out), true, "UTF-8");
        // Anything else printed to stdout would corrupt the protocol.
        System.setOut(new PrintStream(OutputStream.nullOutputStream()));
        if (compiler == null) {
            out.println("{\"fatal\":\"this Java has no compiler (it is a JRE, not a JDK)\"}");
            return;
        }
        BufferedReader in = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
        out.println("{\"ready\":true,\"java\":" + PwJson.quote(System.getProperty("java.version")) + "}");
        String line;
        while ((line = in.readLine()) != null) {
            if (line.equals("PING")) { out.println("{\"pong\":true}"); continue; }
            if (!line.startsWith("COMPILE\t")) { out.println("{\"fatal\":\"bad request\"}"); continue; }
            String[] parts = line.split("\t");
            String fileName = parts[1];
            String outDir = parts[2].equals("-") ? scratch : parts[2];
            int n = Integer.parseInt(parts[3]);
            char[] buf = new char[n];
            int got = 0;
            while (got < n) {
                int r = in.read(buf, got, n - got);
                if (r < 0) return;
                got += r;
            }
            String source = new String(buf);
            try {
                out.println(compile(compiler, fileName, source, outDir, supportCp));
            } catch (Throwable t) {
                out.println("{\"fatal\":" + PwJson.quote("compiler crashed: " + t) + "}");
            }
        }
    }

    private static String compile(JavaCompiler compiler, String fileName, String source, String outDir, String cp) {
        long start = System.nanoTime();
        DiagnosticCollector<JavaFileObject> diags = new DiagnosticCollector<>();
        JavaFileObject file = new SimpleJavaFileObject(URI.create("string:///" + fileName), JavaFileObject.Kind.SOURCE) {
            @Override public CharSequence getCharContent(boolean ignore) { return source; }
        };
        List<String> opts = Arrays.asList(
            "-proc:none", "-Xlint:none", "-g", "-encoding", "UTF-8", "-Xmaxerrs", "50",
            "-cp", cp, "-d", outDir, "-implicit:none");
        StringWriter extra = new StringWriter();
        boolean ok;
        try (StandardJavaFileManager fm = compiler.getStandardFileManager(diags, Locale.ROOT, StandardCharsets.UTF_8)) {
            ok = compiler.getTask(extra, fm, diags, opts, null, Collections.singletonList(file)).call();
        } catch (IOException e) {
            return "{\"fatal\":" + PwJson.quote(e.toString()) + "}";
        }
        StringBuilder b = new StringBuilder();
        b.append("{\"ok\":").append(ok).append(",\"ms\":").append((System.nanoTime() - start) / 1_000_000).append(",\"diagnostics\":[");
        boolean first = true;
        for (Diagnostic<? extends JavaFileObject> d : diags.getDiagnostics()) {
            String sev;
            switch (d.getKind()) {
                case ERROR: sev = "error"; break;
                case WARNING: case MANDATORY_WARNING: sev = "warning"; break;
                default: continue; // notes like "uses unchecked operations"
            }
            if (!first) b.append(',');
            first = false;
            b.append("{\"severity\":\"").append(sev).append('"')
                .append(",\"line\":").append(d.getLineNumber())
                .append(",\"column\":").append(d.getColumnNumber())
                .append(",\"start\":").append(d.getStartPosition())
                .append(",\"end\":").append(d.getEndPosition())
                .append(",\"code\":").append(PwJson.quote(String.valueOf(d.getCode())))
                .append(",\"message\":").append(PwJson.quote(d.getMessage(Locale.ROOT)))
                .append('}');
        }
        b.append("],\"extra\":").append(PwJson.quote(extra.toString())).append('}');
        return b.toString();
    }
}
