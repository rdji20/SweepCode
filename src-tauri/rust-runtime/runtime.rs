// ---- SweepCode runtime: parses inputs, calls the solution, reports results ----
mod pw {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
    use std::sync::Mutex;

    // --- memory cap: allocations past the limit fail, which aborts with a clear message
    pub static USED: AtomicUsize = AtomicUsize::new(0);
    pub static LIMIT: AtomicUsize = AtomicUsize::new(usize::MAX);
    pub struct Capped;
    fn reserve(n: usize) -> bool {
        let now = USED.fetch_add(n, Relaxed) + n;
        if now > LIMIT.load(Relaxed) {
            USED.fetch_sub(n, Relaxed);
            false
        } else {
            true
        }
    }
    unsafe impl GlobalAlloc for Capped {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            if !reserve(l.size()) { return std::ptr::null_mut(); }
            let p = unsafe { System.alloc(l) };
            if p.is_null() { USED.fetch_sub(l.size(), Relaxed); }
            p
        }
        unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
            if !reserve(l.size()) { return std::ptr::null_mut(); }
            let p = unsafe { System.alloc_zeroed(l) };
            if p.is_null() { USED.fetch_sub(l.size(), Relaxed); }
            p
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            unsafe { System.dealloc(p, l) };
            USED.fetch_sub(l.size(), Relaxed);
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, new_size: usize) -> *mut u8 {
            if new_size > l.size() && !reserve(new_size - l.size()) { return std::ptr::null_mut(); }
            let q = unsafe { System.realloc(p, l, new_size) };
            if q.is_null() {
                if new_size > l.size() { USED.fetch_sub(new_size - l.size(), Relaxed); }
            } else if new_size < l.size() {
                USED.fetch_sub(l.size() - new_size, Relaxed);
            }
            q
        }
    }
    #[global_allocator]
    static ALLOC: Capped = Capped;

    // --- tiny JSON reader
    #[derive(Debug, Clone)]
    pub enum Json { Null, Bool(bool), Num(String), Str(String), Arr(Vec<Json>) }

    struct P<'a> { s: &'a [u8], i: usize }
    impl<'a> P<'a> {
        fn err(&self, m: &str) -> String { format!("{m} at position {}", self.i) }
        fn ws(&mut self) { while self.i < self.s.len() && (self.s[self.i] as char).is_whitespace() { self.i += 1; } }
        fn value(&mut self) -> Result<Json, String> {
            self.ws();
            let c = *self.s.get(self.i).ok_or_else(|| self.err("unexpected end of input"))?;
            match c {
                b'[' => {
                    self.i += 1;
                    let mut v = Vec::new();
                    self.ws();
                    if self.s.get(self.i) == Some(&b']') { self.i += 1; return Ok(Json::Arr(v)); }
                    loop {
                        v.push(self.value()?);
                        self.ws();
                        match self.s.get(self.i) {
                            Some(b',') => self.i += 1,
                            Some(b']') => { self.i += 1; return Ok(Json::Arr(v)); }
                            _ => return Err(self.err("expected ',' or ']'")),
                        }
                    }
                }
                b'"' => {
                    self.i += 1;
                    let mut out = String::new();
                    loop {
                        let c = *self.s.get(self.i).ok_or_else(|| self.err("unterminated string"))?;
                        self.i += 1;
                        match c {
                            b'"' => return Ok(Json::Str(out)),
                            b'\\' => {
                                let e = *self.s.get(self.i).ok_or_else(|| self.err("bad escape"))?;
                                self.i += 1;
                                match e {
                                    b'n' => out.push('\n'), b't' => out.push('\t'), b'r' => out.push('\r'),
                                    b'b' => out.push('\u{8}'), b'f' => out.push('\u{c}'),
                                    b'u' => {
                                        let h = std::str::from_utf8(self.s.get(self.i..self.i + 4).ok_or_else(|| self.err("bad unicode escape"))?).map_err(|_| self.err("bad unicode escape"))?;
                                        let n = u32::from_str_radix(h, 16).map_err(|_| self.err("bad unicode escape"))?;
                                        out.push(char::from_u32(n).unwrap_or('\u{fffd}'));
                                        self.i += 4;
                                    }
                                    other => out.push(other as char),
                                }
                            }
                            _ => {
                                // copy one UTF-8 character
                                let start = self.i - 1;
                                let mut end = self.i;
                                while end < self.s.len() && (self.s[end] & 0xC0) == 0x80 { end += 1; }
                                out.push_str(std::str::from_utf8(&self.s[start..end]).unwrap_or("\u{fffd}"));
                                self.i = end;
                            }
                        }
                    }
                }
                b'n' if self.s[self.i..].starts_with(b"null") => { self.i += 4; Ok(Json::Null) }
                b't' if self.s[self.i..].starts_with(b"true") => { self.i += 4; Ok(Json::Bool(true)) }
                b'f' if self.s[self.i..].starts_with(b"false") => { self.i += 5; Ok(Json::Bool(false)) }
                b'-' | b'+' | b'.' | b'0'..=b'9' => {
                    let start = self.i;
                    while self.i < self.s.len() && matches!(self.s[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') { self.i += 1; }
                    Ok(Json::Num(String::from_utf8_lossy(&self.s[start..self.i]).trim_start_matches('+').to_string()))
                }
                other => Err(self.err(&format!("unexpected character '{}'", other as char))),
            }
        }
    }

    pub fn parse(text: &str) -> Result<Json, String> {
        let mut p = P { s: text.as_bytes(), i: 0 };
        let v = p.value()?;
        p.ws();
        if p.i != p.s.len() { return Err(p.err("unexpected trailing text")); }
        Ok(v)
    }

    pub fn describe(j: &Json) -> String {
        match j {
            Json::Null => "null".into(),
            Json::Bool(b) => format!("boolean {b}"),
            Json::Num(n) => format!("number {n}"),
            Json::Str(s) => format!("string {}", quote(s)),
            Json::Arr(_) => "array".into(),
        }
    }

    // --- input conversion
    pub trait FromJson: Sized { fn from_json(j: &Json) -> Result<Self, String>; }

    fn int(j: &Json, ty: &str) -> Result<i128, String> {
        match j {
            Json::Num(n) if !n.contains(['.', 'e', 'E']) => n.parse::<i128>().map_err(|_| format!("{n} is not a whole number")),
            other => Err(format!("expected {ty}, got {}", describe(other))),
        }
    }
    macro_rules! ints {
        ($($t:ty),*) => {$(
            impl FromJson for $t {
                fn from_json(j: &Json) -> Result<Self, String> {
                    let v = int(j, stringify!($t))?;
                    <$t>::try_from(v).map_err(|_| format!("{v} does not fit in {}", stringify!($t)))
                }
            }
        )*};
    }
    ints!(i8, i16, i32, i64, i128, u8, u16, u32, u64, usize, isize);

    impl FromJson for f64 {
        fn from_json(j: &Json) -> Result<Self, String> {
            match j { Json::Num(n) => n.parse().map_err(|_| format!("bad number {n}")), o => Err(format!("expected a number, got {}", describe(o))) }
        }
    }
    impl FromJson for f32 {
        fn from_json(j: &Json) -> Result<Self, String> { f64::from_json(j).map(|v| v as f32) }
    }
    impl FromJson for bool {
        fn from_json(j: &Json) -> Result<Self, String> {
            match j { Json::Bool(b) => Ok(*b), o => Err(format!("expected true/false, got {}", describe(o))) }
        }
    }
    impl FromJson for char {
        fn from_json(j: &Json) -> Result<Self, String> {
            match j {
                Json::Str(s) if s.chars().count() == 1 => Ok(s.chars().next().unwrap()),
                o => Err(format!("expected a one-letter string like \"a\", got {}", describe(o))),
            }
        }
    }
    impl FromJson for String {
        fn from_json(j: &Json) -> Result<Self, String> {
            match j { Json::Str(s) => Ok(s.clone()), o => Err(format!("expected a string in double quotes, got {}", describe(o))) }
        }
    }
    impl<T: FromJson> FromJson for Vec<T> {
        fn from_json(j: &Json) -> Result<Self, String> {
            match j {
                Json::Arr(v) => v.iter().map(T::from_json).collect(),
                o => Err(format!("expected an array, got {}", describe(o))),
            }
        }
    }
    impl FromJson for Json {
        fn from_json(j: &Json) -> Result<Self, String> { Ok(j.clone()) }
    }

    /// Parses one raw input line into the parameter's type.
    pub fn arg<T: FromJson>(raw: &str, name: &str) -> Result<T, String> {
        let j = parse(raw).map_err(|e| format!("{name}: {e}"))?;
        T::from_json(&j).map_err(|e| format!("{name}: {e}"))
    }
    /// One argument of a design-problem call.
    pub fn call_arg<T: FromJson>(args: &[Json], i: usize, name: &str, call: &str) -> Result<T, String> {
        let j = args.get(i).ok_or_else(|| format!("{call} is missing argument {}", i + 1))?;
        T::from_json(j).map_err(|e| format!("{name} in {call}: {e}"))
    }

    // --- output, LeetCode style
    pub trait ToJson { fn to_json(&self) -> String; }
    macro_rules! plain {
        ($($t:ty),*) => {$( impl ToJson for $t { fn to_json(&self) -> String { self.to_string() } } )*};
    }
    plain!(i8, i16, i32, i64, i128, u8, u16, u32, u64, usize, isize, bool);
    impl ToJson for f64 {
        fn to_json(&self) -> String { if self.is_finite() { format!("{:.5}", self) } else { self.to_string() } }
    }
    impl ToJson for f32 { fn to_json(&self) -> String { (*self as f64).to_json() } }
    impl ToJson for char { fn to_json(&self) -> String { quote(&self.to_string()) } }
    impl ToJson for String { fn to_json(&self) -> String { quote(self) } }
    impl ToJson for &str { fn to_json(&self) -> String { quote(self) } }
    impl ToJson for () { fn to_json(&self) -> String { "null".into() } }
    impl<T: ToJson> ToJson for Vec<T> {
        fn to_json(&self) -> String { format!("[{}]", self.iter().map(|x| x.to_json()).collect::<Vec<_>>().join(",")) }
    }
    impl<T: ToJson> ToJson for [T] {
        fn to_json(&self) -> String { format!("[{}]", self.iter().map(|x| x.to_json()).collect::<Vec<_>>().join(",")) }
    }
    impl<T: ToJson, const N: usize> ToJson for [T; N] {
        fn to_json(&self) -> String { self[..].to_json() }
    }

    pub fn quote(s: &str) -> String {
        let mut o = String::with_capacity(s.len() + 2);
        o.push('"');
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""), '\\' => o.push_str("\\\\"), '\n' => o.push_str("\\n"),
                '\r' => o.push_str("\\r"), '\t' => o.push_str("\\t"),
                c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
                c => o.push(c),
            }
        }
        o.push('"');
        o
    }

    // --- results file (JSON lines, same format as the Java driver)
    static RESULTS: Mutex<Option<std::fs::File>> = Mutex::new(None);
    pub fn emit(line: &str) {
        if let Some(f) = RESULTS.lock().unwrap_or_else(|p| p.into_inner()).as_mut() {
            let _ = writeln!(f, "{line}");
            let _ = f.flush();
        }
    }

    // --- panics: remember message and location instead of printing them
    static PANIC: Mutex<Option<(String, String)>> = Mutex::new(None);
    fn install_hook() {
        std::panic::set_hook(Box::new(|info| {
            let msg = if let Some(s) = info.payload().downcast_ref::<&str>() { s.to_string() }
                else if let Some(s) = info.payload().downcast_ref::<String>() { s.clone() }
                else { "panic".to_string() };
            let loc = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_default();
            *PANIC.lock().unwrap_or_else(|p| p.into_inner()) = Some((msg, loc));
        }));
    }

    extern "C" {
        fn dup(fd: i32) -> i32;
        fn dup2(from: i32, to: i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    const CAPTURE: &str = "case-output.txt";
    const CAP: usize = 64 * 1024;

    pub struct Job { pub header: Vec<(String, String)>, pub inputs: Vec<String> }

    /// Reads args, opens the results file, installs the panic hook. Returns the job.
    pub fn start() -> Job {
        let args: Vec<String> = std::env::args().collect();
        if let Some(mb) = args.get(3).and_then(|s| s.parse::<usize>().ok()) {
            LIMIT.store(mb.saturating_mul(1024 * 1024), Relaxed);
        }
        let f = std::fs::File::create(args.get(2).map(String::as_str).unwrap_or("results.jsonl")).expect("cannot create results file");
        *RESULTS.lock().unwrap() = Some(f);
        install_hook();
        let mut text = String::new();
        std::fs::File::open(args.get(1).map(String::as_str).unwrap_or("job.txt")).expect("cannot open job file").read_to_string(&mut text).expect("cannot read job file");
        let mut header = Vec::new();
        let mut lines = text.lines();
        for l in lines.by_ref() {
            if l == "---" { break; }
            if let Some((k, v)) = l.split_once('=') { header.push((k.to_string(), v.to_string())); }
        }
        let mut inputs: Vec<String> = lines.map(String::from).collect();
        while inputs.last().map(|l| l.trim().is_empty()).unwrap_or(false) { inputs.pop(); }
        emit(&format!("{{\"t\":\"meta\",\"guard\":false,\"rust\":true}}"));
        Job { header, inputs }
    }

    pub fn fatal(msg: &str) {
        emit(&format!("{{\"t\":\"fatal\",\"error\":{}}}", quote(msg)));
    }
    pub fn end() { emit("{\"t\":\"end\"}"); }

    /// Runs one test on a thread with a big stack, capturing everything it prints.
    /// `body` returns Ok((output, ms)) or Err(message) for bad input.
    pub fn run_case(i: usize, body: impl FnOnce() -> Result<(String, f64), String> + Send + 'static) {
        emit(&format!("{{\"t\":\"start\",\"i\":{i}}}"));
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        let cap = std::fs::File::create(CAPTURE).expect("cannot create capture file");
        use std::os::unix::io::AsRawFd;
        let (saved_out, saved_err) = unsafe { (dup(1), dup(2)) };
        unsafe {
            dup2(cap.as_raw_fd(), 1);
            dup2(cap.as_raw_fd(), 2);
        }
        *PANIC.lock().unwrap_or_else(|p| p.into_inner()) = None;
        let started = std::time::Instant::now();
        let joined = std::thread::Builder::new().stack_size(512 << 20).spawn(body).expect("cannot start thread").join();
        let total_ms = started.elapsed().as_secs_f64() * 1000.0;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        unsafe {
            dup2(saved_out, 1);
            dup2(saved_err, 2);
            close(saved_out);
            close(saved_err);
        }
        drop(cap);
        let mut printed = Vec::new();
        let truncated = std::fs::File::open(CAPTURE)
            .map(|f| { let _ = f.take((CAP + 1) as u64).read_to_end(&mut printed); printed.len() > CAP })
            .unwrap_or(false);
        printed.truncate(CAP);
        let stdout = String::from_utf8_lossy(&printed).to_string();
        let tail = format!(",\"stdout\":{},\"stdoutTruncated\":{}", quote(&stdout), truncated);
        match joined {
            Ok(Ok((out, ms))) => emit(&format!("{{\"t\":\"done\",\"i\":{i},\"status\":\"ok\",\"output\":{}{tail},\"ms\":{ms:.3}}}", quote(&out))),
            Ok(Err(msg)) => emit(&format!("{{\"t\":\"done\",\"i\":{i},\"status\":\"error\",\"errorKind\":\"input\",\"error\":{},\"trace\":[]{tail},\"ms\":0}}", quote(&msg))),
            Err(_) => {
                let (msg, loc) = PANIC.lock().unwrap_or_else(|p| p.into_inner()).take().unwrap_or_else(|| ("panic".into(), String::new()));
                let kind = if msg.contains("Operation not permitted") { "blocked" } else { "exception" };
                let trace = if loc.is_empty() { String::new() } else { quote(&loc) };
                emit(&format!("{{\"t\":\"done\",\"i\":{i},\"status\":\"error\",\"errorKind\":\"{kind}\",\"error\":{},\"trace\":[{trace}]{tail},\"ms\":{total_ms:.3}}}", quote(&format!("panicked: {msg}"))));
            }
        }
    }
}
use pw::{FromJson, ToJson};

// ListNode / TreeNode in LeetCode's text format
impl pw::FromJson for Option<Box<ListNode>> {
    fn from_json(j: &pw::Json) -> Result<Self, String> {
        let vals: Vec<i32> = pw::FromJson::from_json(j)?;
        let mut head: Option<Box<ListNode>> = None;
        for v in vals.into_iter().rev() {
            head = Some(Box::new(ListNode { val: v, next: head }));
        }
        Ok(head)
    }
}
impl pw::ToJson for Option<Box<ListNode>> {
    fn to_json(&self) -> String {
        let mut out = Vec::new();
        let mut cur = self.as_ref();
        while let Some(n) = cur {
            out.push(n.val.to_string());
            cur = n.next.as_ref();
        }
        format!("[{}]", out.join(","))
    }
}
impl pw::FromJson for Option<Rc<RefCell<TreeNode>>> {
    fn from_json(j: &pw::Json) -> Result<Self, String> {
        let items = match j {
            pw::Json::Arr(v) => v,
            o => return Err(format!("expected a tree like [1,null,2], got {}", pw::describe(o))),
        };
        let val = |x: &pw::Json| -> Result<Option<i32>, String> {
            match x { pw::Json::Null => Ok(None), other => i32::from_json(other).map(Some) }
        };
        let Some(first) = items.first() else { return Ok(None) };
        let Some(rv) = val(first)? else { return Ok(None) };
        let root = Rc::new(RefCell::new(TreeNode::new(rv)));
        let mut queue = VecDeque::from([root.clone()]);
        let mut i = 1;
        while i < items.len() {
            let Some(node) = queue.pop_front() else { break };
            if let Some(v) = val(&items[i])? {
                let c = Rc::new(RefCell::new(TreeNode::new(v)));
                node.borrow_mut().left = Some(c.clone());
                queue.push_back(c);
            }
            i += 1;
            if i < items.len() {
                if let Some(v) = val(&items[i])? {
                    let c = Rc::new(RefCell::new(TreeNode::new(v)));
                    node.borrow_mut().right = Some(c.clone());
                    queue.push_back(c);
                }
                i += 1;
            }
        }
        Ok(Some(root))
    }
}
impl pw::ToJson for Option<Rc<RefCell<TreeNode>>> {
    fn to_json(&self) -> String {
        let mut out: Vec<String> = Vec::new();
        let mut queue: VecDeque<Option<Rc<RefCell<TreeNode>>>> = VecDeque::from([self.clone()]);
        while let Some(n) = queue.pop_front() {
            match n {
                None => out.push("null".into()),
                Some(n) => {
                    let b = n.borrow();
                    out.push(b.val.to_string());
                    queue.push_back(b.left.clone());
                    queue.push_back(b.right.clone());
                }
            }
            if out.len() > 200_000 { break; }
        }
        while out.last().map(|s| s == "null").unwrap_or(false) { out.pop(); }
        format!("[{}]", out.join(","))
    }
}
