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
  [
    "actor state projection canonical spacing",
    "mtd ( Foo mut self . state . { counter , xyz } ) update(): void { return; }",
    "mtd (Foo mut self.state.{counter, xyz}) update(): void { return; }"
  ],
  [
    "actor state projection already canonical",
    "mtd (Foo mut self.state.{counter, xyz}) update(): void { return; }",
    "mtd (Foo mut self.state.{counter, xyz}) update(): void { return; }"
  ],
  [
    "actor single state field and singleton group",
    "mtd ( Foo mut self . state . { counter } ) increment(): void { return; }",
    "mtd (Foo mut self.state.counter) increment(): void { return; }"
  ],
  [
    "actor whole state authority retained",
    "mtd (Foo mut self.state) reset(): void { return; }",
    "mtd (Foo mut self.state) reset(): void { return; }"
  ],
  [
    "duplicate projected field is not normalized",
    "mtd (Foo mut self.state.{counter,counter}) invalid(): void { return; }",
    "mtd (Foo mut self.state.{counter,counter}) invalid(): void { return; }"
  ],
  [
    "square-bracket receiver is not accepted as projection",
    "mtd (Foo mut [self.state.{counter,xyz}]) invalid(): void { return; }",
    "mtd (Foo mut [self.state.{counter,xyz}]) invalid(): void { return; }"
  ],
  [
    "comment in projection prevents destructive edits",
    "mtd (Foo mut self.state.{counter, /*keep*/ xyz}) update(): void { return; }",
    "mtd (Foo mut self.state.{counter, /*keep*/ xyz}) update(): void { return; }"
  ],
  [
    "noncode projection text is preserved",
    "// mtd (Foo mut self.state.{counter,xyz}) update(): void {}\\nval label = \"mtd (Foo mut self.state.{counter,xyz})\";",
    "// mtd (Foo mut self.state.{counter,xyz}) update(): void {}\\nval label = \"mtd (Foo mut self.state.{counter,xyz})\";"
  ],
  ["named this actor receiver",
    "mtd ( Foo this ) get_id(): int { return 1; }",
    "mtd (Foo this) get_id(): int { return 1; }"],
  ["named node actor receiver",
    "mtd ( Foo node ) get_id(): int { return 1; }",
    "mtd (Foo node) get_id(): int { return 1; }"],
  ["named this typed alias",
    "mtd ( this : Foo ) get_id(): int { return 1; }",
    "mtd (this: Foo) get_id(): int { return 1; }"],
  ["mutable typed alias preserves name",
    "mtd ( mut this : Foo ) get_id(): int { return 1; }",
    "mtd (mut this: Foo) get_id(): int { return 1; }"],
  ["named this projected single field",
    "mtd ( Foo mut this . state . { counter } ) increment(): void { this.handle_state(critical-write |s| -> { s.counter += 1; }); }",
    "mtd (Foo mut this.state.counter) increment(): void { this.handle_state(critical-write |s| -> { s.counter += 1; }); }"],
  ["named this projected group",
    "mtd ( Foo mut this . state . { counter , xyz } ) update(): void { this.handle_state(critical-write |s| -> { s.counter += 1; s.xyz += 1; }); }",
    "mtd (Foo mut this.state.{counter, xyz}) update(): void { this.handle_state(critical-write |s| -> { s.counter += 1; s.xyz += 1; }); }"],
  ["named node whole state",
    "mtd ( Foo mut node . state ) reset(): void { return; }",
    "mtd (Foo mut node.state) reset(): void { return; }"],
  ["named state projection with invalid reserved binding is untouched",
    "mtd (Foo mut actor.state.{counter,xyz}) invalid(): void { return; }",
    "mtd (Foo mut actor.state.{counter,xyz}) invalid(): void { return; }"],
  ["named duplicate projected field is untouched",
    "mtd (Foo mut this.state.{counter,counter}) invalid(): void { return; }",
    "mtd (Foo mut this.state.{counter,counter}) invalid(): void { return; }"],
  ["receiver projection comments are not stripped",
    "mtd ( Foo mut this.state.{counter,/*retain*/xyz} ) f(): void { return; }",
    "mtd ( Foo mut this.state.{counter,/*retain*/xyz} ) f(): void { return; }"],
  ["noncode named receiver text is preserved",
    "// mtd ( Foo mut this . state . counter ) f(): void {}\\nval x = \"mtd (Foo this) f(): void {}\";",
    "// mtd ( Foo mut this . state . counter ) f(): void {}\\nval x = \"mtd (Foo this) f(): void {}\";"],
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
