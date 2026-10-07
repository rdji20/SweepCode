package javafx.util;

/** LeetCode's Java environment ships javafx.util.Pair; the plain JDK does not. */
public class Pair<K, V> implements java.io.Serializable {
    private final K key;
    private final V value;
    public Pair(K key, V value) { this.key = key; this.value = value; }
    public K getKey() { return key; }
    public V getValue() { return value; }
    @Override public String toString() { return key + "=" + value; }
    @Override public int hashCode() { return java.util.Objects.hash(key, value); }
    @Override public boolean equals(Object o) {
        if (this == o) return true;
        if (!(o instanceof Pair)) return false;
        Pair<?, ?> p = (Pair<?, ?>) o;
        return java.util.Objects.equals(key, p.key) && java.util.Objects.equals(value, p.value);
    }
}
