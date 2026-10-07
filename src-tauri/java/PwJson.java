import java.math.BigInteger;
import java.util.ArrayList;
import java.util.List;

/** Tiny JSON reader/writer used by the driver and the checker. No dependencies. */
final class PwJson {
    private final String s;
    private int i;

    private PwJson(String s) { this.s = s; }

    /** Parses one JSON value. Integers become Long (or BigInteger), decimals Double, arrays List. */
    static Object parse(String text) {
        PwJson p = new PwJson(text);
        p.ws();
        Object v = p.value();
        p.ws();
        if (p.i != p.s.length()) throw p.err("unexpected trailing text");
        return v;
    }

    private IllegalArgumentException err(String msg) {
        String near = s.substring(Math.max(0, i - 10), Math.min(s.length(), i + 10));
        return new IllegalArgumentException(msg + " at position " + i + " near '" + near + "'");
    }

    private void ws() { while (i < s.length() && Character.isWhitespace(s.charAt(i))) i++; }

    private Object value() {
        if (i >= s.length()) throw err("unexpected end of input");
        char c = s.charAt(i);
        if (c == '[') return array();
        if (c == '"') return string();
        if (c == '{') throw err("objects are not supported");
        if (s.startsWith("null", i)) { i += 4; return null; }
        if (s.startsWith("true", i)) { i += 4; return Boolean.TRUE; }
        if (s.startsWith("false", i)) { i += 5; return Boolean.FALSE; }
        if (c == '-' || c == '+' || (c >= '0' && c <= '9') || c == '.') return number();
        throw err("unexpected character '" + c + "'");
    }

    private List<Object> array() {
        List<Object> out = new ArrayList<>();
        i++;
        ws();
        if (i < s.length() && s.charAt(i) == ']') { i++; return out; }
        while (true) {
            ws();
            out.add(value());
            ws();
            if (i >= s.length()) throw err("unterminated array");
            char c = s.charAt(i++);
            if (c == ']') return out;
            if (c != ',') throw err("expected ',' or ']'");
        }
    }

    private String string() {
        StringBuilder b = new StringBuilder();
        i++;
        while (true) {
            if (i >= s.length()) throw err("unterminated string");
            char c = s.charAt(i++);
            if (c == '"') return b.toString();
            if (c != '\\') { b.append(c); continue; }
            if (i >= s.length()) throw err("bad escape");
            char e = s.charAt(i++);
            switch (e) {
                case '"': b.append('"'); break;
                case '\\': b.append('\\'); break;
                case '/': b.append('/'); break;
                case 'b': b.append('\b'); break;
                case 'f': b.append('\f'); break;
                case 'n': b.append('\n'); break;
                case 'r': b.append('\r'); break;
                case 't': b.append('\t'); break;
                case 'u':
                    if (i + 4 > s.length()) throw err("bad unicode escape");
                    b.append((char) Integer.parseInt(s.substring(i, i + 4), 16));
                    i += 4;
                    break;
                default: throw err("bad escape \\" + e);
            }
        }
    }

    private Object number() {
        int start = i;
        if (s.charAt(i) == '-' || s.charAt(i) == '+') i++;
        boolean frac = false;
        while (i < s.length()) {
            char c = s.charAt(i);
            if (c >= '0' && c <= '9') { i++; continue; }
            if (c == '.' || c == 'e' || c == 'E' || ((c == '-' || c == '+') && (s.charAt(i - 1) == 'e' || s.charAt(i - 1) == 'E'))) {
                frac = true; i++; continue;
            }
            break;
        }
        String t = s.substring(start, i);
        if (t.startsWith("+")) t = t.substring(1);
        try {
            if (frac) return Double.parseDouble(t);
            BigInteger big = new BigInteger(t);
            if (big.bitLength() < 64) return big.longValue();
            return big;
        } catch (NumberFormatException ex) {
            throw err("bad number '" + t + "'");
        }
    }

    /** JSON string literal with escapes. */
    static String quote(String v) {
        StringBuilder b = new StringBuilder(v.length() + 2);
        b.append('"');
        for (int k = 0; k < v.length(); k++) {
            char c = v.charAt(k);
            switch (c) {
                case '"': b.append("\\\""); break;
                case '\\': b.append("\\\\"); break;
                case '\n': b.append("\\n"); break;
                case '\r': b.append("\\r"); break;
                case '\t': b.append("\\t"); break;
                case '\b': b.append("\\b"); break;
                case '\f': b.append("\\f"); break;
                default:
                    if (c < 0x20) b.append(String.format("\\u%04x", (int) c));
                    else b.append(c);
            }
        }
        return b.append('"').toString();
    }
}
