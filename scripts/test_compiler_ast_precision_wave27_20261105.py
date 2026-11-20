#!/usr/bin/env python3
"""Source-local semantic AST admission regressions; no compiler execution."""
import unittest
from decimal import Decimal

from check_compiler_ast_parity import InvalidAst, parse_ast, semantic_ast_sha256, typed_ast_signature


class SemanticAstPrecisionWave27Tests(unittest.TestCase):
    def test_true_int_and_decimal_are_distinct(self):
        variants = [{"value": True}, {"value": 1}, {"value": Decimal("1.0")}]
        signatures = [typed_ast_signature(v) for v in variants]
        self.assertEqual(len(set(signatures)), 3)
        self.assertEqual(len({semantic_ast_sha256(v) for v in variants}), 3)

    def test_key_reordering_has_same_digest_but_array_order_does_not(self):
        self.assertEqual(
            semantic_ast_sha256({"a": 1, "b": [True, Decimal("1.01")]}),
            semantic_ast_sha256({"b": [True, Decimal("1.01")], "a": 1}),
        )
        self.assertNotEqual(
            semantic_ast_sha256({"values": [1, True]}),
            semantic_ast_sha256({"values": [True, 1]}),
        )

    def test_escaped_duplicate_keys_and_nonfinite_literals_fail_closed(self):
        invalid = [
            b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":1,"\\u0078":2}}',
            b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":NaN}}',
            b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":1e999}}',
            b'{"schema":"oreslang-semantic-ast-v1","ast":{}}',
            b'{"schema":"other","ast":{"x":1}}',
        ]
        for value in invalid:
            with self.subTest(value=value):
                with self.assertRaises(InvalidAst):
                    parse_ast(value)

    def test_high_precision_decimal_is_not_rounded_to_binary_float(self):
        a = parse_ast(
            b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":1.000000000000000000000001}}'
        )
        b = parse_ast(
            b'{"schema":"oreslang-semantic-ast-v1","ast":{"x":1.000000000000000000000002}}'
        )
        self.assertNotEqual(typed_ast_signature(a), typed_ast_signature(b))
        self.assertNotEqual(semantic_ast_sha256(a), semantic_ast_sha256(b))


if __name__ == "__main__":
    unittest.main()
