"""Typed semantic AST hashes must reject JSON ambiguity and number coercion."""
from decimal import Decimal
import json
import unittest
from check_compiler_ast_parity import (
    InvalidAst, SCHEMA, parse_ast, semantic_ast_sha256, typed_ast_signature,
)


def encoded(value):
    return json.dumps({"schema": SCHEMA, "ast": value},
                      separators=(",", ":")).encode("utf-8")


class TypedAstSignatureWave24(unittest.TestCase):
    def test_object_field_order_does_not_change_semantics(self):
        first = parse_ast(encoded({"flag": True, "value": 1}))
        second = parse_ast(encoded({"value": 1, "flag": True}))
        self.assertEqual(semantic_ast_sha256(first), semantic_ast_sha256(second))

    def test_bool_int_decimal_and_string_have_distinct_signatures(self):
        values = [True, 1, Decimal("1.0"), "1"]
        hashes = {semantic_ast_sha256({"value": item}) for item in values}
        self.assertEqual(len(hashes), len(values))

    def test_decimal_precision_does_not_round_to_binary_float(self):
        first = parse_ast(b'{"schema":"oreslang-semantic-ast-v1","ast":{"n":1.000000000000000000001}}')
        second = parse_ast(b'{"schema":"oreslang-semantic-ast-v1","ast":{"n":1.000000000000000000002}}')
        self.assertNotEqual(semantic_ast_sha256(first), semantic_ast_sha256(second))

    def test_array_order_remains_semantically_significant(self):
        self.assertNotEqual(semantic_ast_sha256([1, 2]), semantic_ast_sha256([2, 1]))

    def test_duplicate_ast_key_is_rejected(self):
        with self.assertRaises(InvalidAst):
            parse_ast(b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":1,"x":2}}')

    def test_duplicate_envelope_key_is_rejected(self):
        with self.assertRaises(InvalidAst):
            parse_ast(b'{"schema":"oreslang-semantic-ast-v1","schema":"oreslang-semantic-ast-v1","ast":{"x":1}}')

    def test_nonfinite_and_excessively_large_exponents_fail_closed(self):
        for literal in (b"NaN", b"Infinity", b"-Infinity", b"1e309"):
            with self.subTest(literal=literal):
                raw = b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":' + literal + b'}}'
                with self.assertRaises(InvalidAst):
                    parse_ast(raw)

    def test_extra_envelope_keys_and_empty_ast_rejected(self):
        for raw in (b'{"schema":"oreslang-semantic-ast-v1","ast":{}}',
                    b'{"schema":"oreslang-semantic-ast-v1","ast":[]}',
                    b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":1},"extra":0}'):
            with self.subTest(raw=raw):
                with self.assertRaises(InvalidAst):
                    parse_ast(raw)

    def test_invalid_utf8_json_must_not_be_replaced(self):
        with self.assertRaises(InvalidAst):
            parse_ast(b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":"\xff"}}')


if __name__ == "__main__":
    unittest.main(verbosity=2)
