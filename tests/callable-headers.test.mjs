import test from 'node:test';
import assert from 'node:assert/strict';
import {normalize_callable_headers} from '../src/callable-headers.mjs';
const cases = [
  [
    "named legacy",
    "mtd (Calculator self) square(int x) -> int :: x*x end",
    "mtd (self: Calculator) square(int x) -> int :: x*x end"
  ],
  [
    "named already canonical",
    "mtd (self: Calculator) square(int x) -> int :: x*x end",
    "mtd (self: Calculator) square(int x) -> int :: x*x end"
  ],
  [
    "implicit underscore",
    "mtd (Calculator _) square(int x) -> int :: x*x end",
    "mtd (Calculator) square(int x) -> int :: x*x end"
  ],
  [
    "implicit already canonical",
    "mtd (Calculator) square(int x) -> int :: x*x end",
    "mtd (Calculator) square(int x) -> int :: x*x end"
  ],
  [
    "mutable explicit",
    "mtd ( mut self : Calculator ) square(int x) -> int :: x*x end",
    "mtd (mut self: Calculator) square(int x) -> int :: x*x end"
  ],
  [
    "static and 5 inline",
    "define class C as\n  fnc a(): int { return 5; }\n  method b(): int -> { return 5; }\n  c() -> int { return 5; }\n  fnc d() -> int\n    let int x = 3,\n    x + 2\n  end\n  method e() -> int :: let int x = 3, x + 2 end\n  static fnc s(): int { return 5; }\nend",
    "define class C as\n  a(): int { return 5; }\n  b(): int -> { return 5; }\n  c() -> int { return 5; }\n  d() -> int\n    let int x = 3,\n    x + 2\n  end\n  e() -> int :: let int x = 3, x + 2 end\n  static fnc s(): int { return 5; }\nend"
  ],
  [
    "generic inline methods, nested return types, and static fnc",
    "define class Box as\n  fnc identity<T>(T value): T { return value; }\n  method wrap<T, E=Error>(T value) -> Result<Option<T>, E> { return value; }\n  fnc tail<T>() -> T :: make<T>() end\n  static fnc keep<T>(T value): T { return value; }\n  fnc after(): int { return 5; }\nend",
    "define class Box as\n  identity<T>(T value): T { return value; }\n  wrap<T, E=Error>(T value) -> Result<Option<T>, E> { return value; }\n  tail<T>() -> T :: make<T>() end\n  static fnc keep<T>(T value): T { return value; }\n  after(): int { return 5; }\nend"
  ],
  [
    "bare generic method does not hide next prefixed method",
    "define class C as\n  identity<T>(T x) -> T { return x; }\n  fnc next(): int { return 1; }\nend",
    "define class C as\n  identity<T>(T x) -> T { return x; }\n  next(): int { return 1; }\nend"
  ],
  [
    "generic callable return types and qualified names",
    "define actor A as\n  fnc f<T>(T value) -> fnc(int)->T { return value; }\n  method g<T>() : my.ns.Box<Option<T>> { return 0; }\nend",
    "define actor A as\n  f<T>(T value) -> fnc(int)->T { return value; }\n  g<T>() : my.ns.Box<Option<T>> { return 0; }\nend"
  ],
  [
    "unclosed generic method not rewritten",
    "define class C as fnc broken<T(T value): T { return value; } end",
    "define class C as fnc broken<T(T value): T { return value; } end"
  ],
  [
    "comment between keyword and generic method name blocks rewrite",
    "define class C as fnc /*keep*/ id<T>(T x): T { return x; } end",
    "define class C as fnc /*keep*/ id<T>(T x): T { return x; } end"
  ],
  [
    "noncode must be untouched",
    "// mtd (Calculator self) foo() -> int :: 5 end\nfnc foo(): int { return 5; }\nval x = \"mtd (Calculator self) foo()\";\nmtd (Calculator /*keep*/ self) bar() -> int :: 5 end",
    "// mtd (Calculator self) foo() -> int :: 5 end\nfnc foo(): int { return 5; }\nval x = \"mtd (Calculator self) foo()\";\nmtd (Calculator /*keep*/ self) bar() -> int :: 5 end"
  ],
  [
    "invalid receiver untouched",
    "mtd (Calculator mut self) set() -> int :: 5 end",
    "mtd (Calculator mut self) set() -> int :: 5 end"
  ],
  ["constructor adjacent groups","pub constructor (Counter)(int initial): self { self.value = initial; }","pub constructor (Counter)(int initial): self { self.value = initial; }"],
  ["constructor spaced groups","pub constructor (Counter) (int initial): self { self.value = initial; }","pub constructor (Counter)(int initial): self { self.value = initial; }"],
  ["constructor compact keyword","pub constructor(Counter) (int initial): self { self.value = initial; }","pub constructor (Counter)(int initial): self { self.value = initial; }"],
  ["constructor fully compact","pub constructor(Counter)(int initial): self { self.value = initial; }","pub constructor (Counter)(int initial): self { self.value = initial; }"],
  ["constructor multiline spacing","pub constructor\n (Counter)\n (int initial): self { self.value = initial; }","pub constructor (Counter)(int initial): self { self.value = initial; }"],
  ["constructor comment before owner retained","pub constructor/*keep*/(Counter) (int initial): self { self.value = initial; }","pub constructor/*keep*/(Counter) (int initial): self { self.value = initial; }"],
  ["constructor comment between groups retained","pub constructor (Counter) /*keep*/ (int initial): self { self.value = initial; }","pub constructor (Counter) /*keep*/ (int initial): self { self.value = initial; }"],
  ["incomplete constructor header untouched","constructor(Counter) (int initial): self","constructor(Counter) (int initial): self"],
  ["invalid constructor result untouched","constructor(Counter) (int initial): Counter {}","constructor(Counter) (int initial): Counter {}"],
  ["inline class constructor untouched","define class Counter as constructor(int initial) { self.value = initial; } end","define class Counter as constructor(int initial) { self.value = initial; } end"],
  ["noncode constructor text untouched","// constructor(Counter) (int n): self {}\nval s = \"constructor(Counter) (int n): self {}\";","// constructor(Counter) (int n): self {}\nval s = \"constructor(Counter) (int n): self {}\";"],
  ["member constructor calls untouched","obj.constructor(Counter) (int initial): self {}","obj.constructor(Counter) (int initial): self {}"],
  ["backtick template constructor text untouched", "val t = `constructor(Counter) (int initial): self {}`;", "val t = `constructor(Counter) (int initial): self {}`;"]
];
for (const [name,input,expected] of cases) {
  test(name, () => {
    const output = normalize_callable_headers(input);
    assert.equal(output, expected);
    assert.equal(normalize_callable_headers(output), output, 'format twice equals format once');
  });
}

test('canonical public generic class factory and methods omit fnc, free and static functions retain it', () => {
  const input = `define pub class Container[T]
    implements Newable[Container[T]] as
  pub fnc new(): Container[T] { return new Container[T](); }
  val int count = 21;
  val bool ready = true;
  val string label = "fnc fake()";
  fnc hidden(T value): T { return value; }
  pub fnc echo(T value): T { return value; }
  pub fnc get_count(): int { return self.count; }
  pub static fnc helper(): int { return 42; }
end
pub fnc main(): int { return 42; }`;
  const expected = input.replace('pub fnc new(', 'pub new(')
    .replace('fnc hidden(', 'hidden(').replace('pub fnc echo(', 'pub echo(')
    .replace('pub fnc get_count(', 'pub get_count(');
  assert.equal(normalize_callable_headers(input), expected);
  assert.equal(normalize_callable_headers(expected), expected);
});
test('unclosed generic header cannot capture later free callable', () => {
  const input = 'define pub class Box[T implements Newable[Box[T]] as\nend\npub fnc main(): int { return 42; }';
  assert.equal(normalize_callable_headers(input), input);
});
test('generic factory comment boundary remains unchanged', () => {
  const input = 'define pub class Box[T] implements Newable[Box[T]] as pub fnc /* keep */ new(): Box[T] { return new Box[T](); } end';
  assert.equal(normalize_callable_headers(input), input);
});
