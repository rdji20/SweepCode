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

export const TEMPLATES: Template[] = [
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

/** Matches "heap", "2d", "hash map", "pq"... Best matches first. */
export function searchTemplates(query: string, list: Template[] = TEMPLATES): Template[] {
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
