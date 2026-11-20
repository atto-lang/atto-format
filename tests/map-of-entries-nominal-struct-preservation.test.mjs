import test from "node:test";
import assert from "node:assert/strict";
import {normalize_callable_headers} from "../src/callable-headers.mjs";

// Current atto-format is deliberately a compiler-verified callable-header
// normalizer, NOT a general language parser/formatter. Never rewrite these
// typed constructor expressions or nominate the formatter as spec authority.
// Full grammar/canonical printer needs a separate conformance implementation.
const mustPreserve = [
  'const m = Map.of_entries<int, bool>(5 => true, 8 => false, 9 => true);',
  'const m = Map.of_entries<int, bool>(Entry.of(5, true), 8 => false);',
  'val m = Map.of_entries<string, int>("a" => 40, "b" => 2);',
  'val m = Map.of(5, true, 8, false);',
  'val Pair<int,bool> p = 5 => true;',
  'val Entry<int,bool> e = 5 => true;',
  'define struct Point as\n  int x;\n  int y;\nend',
  'val Point p = Point{y: 2, x: 40};',
  'val p = infer struct{x: 40, y: 2};',
  'val Record r = (x=40, y=2);',
  '// Map.of_entries<int,bool>(5=>true)\nval x = "Map.of_entries<int,bool>(5=>true)";',
  'define struct First as int x; end\ndefine struct Second as bool x; end',
  'val Map<int,string> empty = {};',
];
for(const [i,source] of mustPreserve.entries()){
  test(`typed Entry and nominal Struct lexical identity #${i}`,()=>{
    const output=normalize_callable_headers(source);
    assert.equal(output,source,"unsupported aggregate syntax must remain unchanged");
    assert.equal(normalize_callable_headers(output),output,"formatting must be idempotent");
  });
}
test("verified callable rewrite preserves nested Entry literal and struct shape",()=>{
  const source=[
    "define class Builder as",
    "  fnc make(): int {",
    "    const m = Map.of_entries<int,bool>(5 => true, 8 => false);",
    "    return m[5] ? 42 : 0;",
    "  }",
    "end",
  ].join("\n");
  const result=normalize_callable_headers(source);
  assert.equal(result,source.replace("  fnc make()", "  make()"));
  assert.ok(result.includes("Map.of_entries<int,bool>(5 => true, 8 => false)"));
  assert.equal(normalize_callable_headers(result),result);
});
