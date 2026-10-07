// Declaration templates: syntax only, never logic.
// Type the prefix and press Tab, or press ⌃Space / ⌘I to pick from the list.
// ${1:name} is a slot you can Tab through; $0 is where the cursor ends.

export interface Template {
  prefix: string;
  label: string;
  body: string;
  /** Extra words the ⌘I picker matches on. */
  keywords?: string;
}

export const JAVA_TEMPLATES: Template[] = [
  // arrays
  { prefix: "arr", label: "int array of size n", body: "${1:int}[] ${2:arr} = new ${1:int}[${3:n}];$0", keywords: "array new sized fixed" },
  { prefix: "arrv", label: "array with values", body: "${1:int}[] ${2:arr} = {${3:1, 2, 3}};$0", keywords: "array literal initialize values" },
  { prefix: "arr2", label: "2D array", body: "${1:int}[][] ${2:grid} = new ${1:int}[${3:rows}][${4:cols}];$0", keywords: "2d matrix grid two dimensional board" },
  { prefix: "dp", label: "dp array (n + 1)", body: "${1:int}[] ${2:dp} = new ${1:int}[${3:n} + 1];$0", keywords: "dynamic programming memo table" },
  // lists
  { prefix: "list", label: "List / ArrayList", body: "List<${1:Integer}> ${2:list} = new ArrayList<>();$0", keywords: "arraylist dynamic array vector" },
  { prefix: "list2", label: "List of lists", body: "List<List<${1:Integer}>> ${2:result} = new ArrayList<>();$0", keywords: "nested lists result answer combinations" },
  { prefix: "listof", label: "List with values", body: "List<${1:Integer}> ${2:list} = new ArrayList<>(Arrays.asList(${3:1, 2, 3}));$0", keywords: "arrays aslist initialize values" },
  // maps and sets
  { prefix: "map", label: "Map / HashMap", body: "Map<${1:Integer}, ${2:Integer}> ${3:map} = new HashMap<>();$0", keywords: "hashmap dictionary hash table frequency" },
  { prefix: "maplist", label: "Map to list", body: "Map<${1:Integer}, List<${2:Integer}>> ${3:graph} = new HashMap<>();$0", keywords: "graph adjacency list neighbors" },
  { prefix: "tmap", label: "TreeMap (sorted keys)", body: "TreeMap<${1:Integer}, ${2:Integer}> ${3:map} = new TreeMap<>();$0", keywords: "sorted ordered treemap floor ceiling" },
  { prefix: "set", label: "Set / HashSet", body: "Set<${1:Integer}> ${2:seen} = new HashSet<>();$0", keywords: "hashset visited seen unique" },
  { prefix: "tset", label: "TreeSet (sorted)", body: "TreeSet<${1:Integer}> ${2:set} = new TreeSet<>();$0", keywords: "sorted ordered treeset floor ceiling" },
  // stacks, queues, heaps
  { prefix: "stack", label: "stack (ArrayDeque)", body: "Deque<${1:Integer}> ${2:stack} = new ArrayDeque<>();$0", keywords: "lifo deque push pop monotonic" },
  { prefix: "queue", label: "queue (ArrayDeque)", body: "Queue<${1:Integer}> ${2:queue} = new ArrayDeque<>();$0", keywords: "fifo bfs offer poll" },
  { prefix: "pq", label: "min-heap", body: "PriorityQueue<${1:Integer}> ${2:pq} = new PriorityQueue<>();$0", keywords: "priority queue min heap smallest" },
  { prefix: "pqmax", label: "max-heap", body: "PriorityQueue<${1:Integer}> ${2:pq} = new PriorityQueue<>(Collections.reverseOrder());$0", keywords: "priority queue max heap largest reverse" },
  { prefix: "pqarr", label: "heap of int[] by index", body: "PriorityQueue<int[]> ${1:pq} = new PriorityQueue<>((a, b) -> a[${2:0}] - b[${2:0}]);$0", keywords: "priority queue comparator pairs dijkstra intervals" },
  // strings
  { prefix: "sb", label: "StringBuilder", body: "StringBuilder ${1:sb} = new StringBuilder();$0", keywords: "string builder append" },
  { prefix: "chars", label: "String to char array", body: "char[] ${1:chars} = ${2:s}.toCharArray();$0", keywords: "char array string characters" },
  { prefix: "count", label: "letter counts (26)", body: "int[] ${1:count} = new int[26];$0", keywords: "frequency letters alphabet counts" },
];

/** The template as it reads once inserted with its default values. */
export function preview(t: Template): string {
  return t.body.replace(/\$\{\d+:([^}]*)\}/g, "$1").replace(/\$\d+/g, "");
}

/** Rust: the declarations and idioms people forget most on LeetCode. */
export const RUST_TEMPLATES: Template[] = [
  // vectors
  { prefix: "vec", label: "empty Vec", body: "let mut ${2:v}: Vec<${1:i32}> = Vec::new();$0", keywords: "vector array list dynamic" },
  { prefix: "arr", label: "Vec of size n", body: "let mut ${1:arr} = vec![${2:0}; ${3:n}];$0", keywords: "array vector sized fill" },
  { prefix: "arrv", label: "Vec with values", body: "let ${1:arr} = vec![${2:1, 2, 3}];$0", keywords: "array literal initialize values" },
  { prefix: "arr2", label: "2D grid", body: "let mut ${1:grid} = vec![vec![${2:0}; ${4:cols}]; ${3:rows}];$0", keywords: "2d matrix grid two dimensional board" },
  { prefix: "dp", label: "dp table (n + 1)", body: "let mut ${1:dp} = vec![${2:0}; ${3:n} + 1];$0", keywords: "dynamic programming memo table" },
  // maps and sets
  { prefix: "map", label: "HashMap", body: "let mut ${3:map}: HashMap<${1:i32}, ${2:i32}> = HashMap::new();$0", keywords: "hashmap dictionary hash table frequency" },
  { prefix: "entry", label: "count with entry()", body: "*${1:map}.entry(${2:key}).or_insert(0) += 1;$0", keywords: "frequency count increment or insert default" },
  { prefix: "maplist", label: "HashMap to Vec (graph)", body: "let mut ${3:graph}: HashMap<${1:i32}, Vec<${2:i32}>> = HashMap::new();$0", keywords: "graph adjacency list neighbors" },
  { prefix: "tmap", label: "BTreeMap (sorted keys)", body: "let mut ${3:map}: BTreeMap<${1:i32}, ${2:i32}> = BTreeMap::new();$0", keywords: "sorted ordered treemap range" },
  { prefix: "set", label: "HashSet", body: "let mut ${2:seen}: HashSet<${1:i32}> = HashSet::new();$0", keywords: "hashset visited seen unique" },
  { prefix: "tset", label: "BTreeSet (sorted)", body: "let mut ${2:set}: BTreeSet<${1:i32}> = BTreeSet::new();$0", keywords: "sorted ordered treeset range" },
  // stacks, queues, heaps
  { prefix: "stack", label: "stack (Vec)", body: "let mut ${2:stack}: Vec<${1:i32}> = Vec::new();$0", keywords: "lifo push pop monotonic" },
  { prefix: "queue", label: "queue (VecDeque)", body: "let mut ${2:queue}: VecDeque<${1:i32}> = VecDeque::new();$0", keywords: "fifo bfs push_back pop_front deque" },
  { prefix: "pq", label: "min-heap", body: "let mut ${2:pq}: BinaryHeap<Reverse<${1:i32}>> = BinaryHeap::new();$0", keywords: "priority queue min heap smallest reverse" },
  { prefix: "pqmax", label: "max-heap", body: "let mut ${2:pq}: BinaryHeap<${1:i32}> = BinaryHeap::new();$0", keywords: "priority queue max heap largest" },
  { prefix: "pqarr", label: "min-heap of (cost, node)", body: "let mut ${1:pq}: BinaryHeap<Reverse<(${2:i32}, usize)>> = BinaryHeap::new();$0", keywords: "priority queue tuple pairs dijkstra" },
  // strings
  { prefix: "sb", label: "String builder", body: "let mut ${1:s} = String::new();$0", keywords: "string builder push_str append" },
  { prefix: "chars", label: "String to Vec<char>", body: "let ${1:chars}: Vec<char> = ${2:s}.chars().collect();$0", keywords: "char array string characters" },
  { prefix: "bytes", label: "String as bytes", body: "let ${1:b} = ${2:s}.as_bytes();$0", keywords: "bytes u8 ascii index" },
  { prefix: "count", label: "letter counts (26)", body: "let mut ${1:count} = [0usize; 26];$0", keywords: "frequency letters alphabet counts" },
  // LeetCode node types
  { prefix: "node", label: "new list node", body: "Some(Box::new(ListNode::new(${1:0})))$0", keywords: "linked list listnode box" },
  { prefix: "tree", label: "new tree node", body: "Some(Rc::new(RefCell::new(TreeNode::new(${1:0}))))$0", keywords: "binary tree treenode rc refcell" },
];

/** Back-compat name for the Java list. */
export const TEMPLATES = JAVA_TEMPLATES;

export function templatesFor(lang: string): Template[] {
  return lang === "rust" ? RUST_TEMPLATES : JAVA_TEMPLATES;
}

/** Matches "heap", "2d", "hash map", "pq"... Best matches first. */
export function searchTemplates(query: string, list: Template[] = JAVA_TEMPLATES): Template[] {
  const q = query.trim().toLowerCase();
  if (!q) return list;
  const words = q.split(/\s+/);
  const scored: { t: Template; score: number }[] = [];
  for (const t of list) {
    const hay = `${t.prefix} ${t.label} ${t.keywords ?? ""} ${preview(t)}`.toLowerCase();
    if (!words.every((w) => hay.includes(w))) continue;
    let score = 3;
    if (t.prefix === q) score = 0;
    else if (t.prefix.startsWith(q)) score = 1;
    else if (t.label.toLowerCase().includes(q)) score = 2;
    scored.push({ t, score });
  }
  return scored.sort((a, b) => a.score - b.score).map((x) => x.t);
}
