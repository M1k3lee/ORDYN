module.exports = grammar({
  name: 'ordyn',

  extras: $ => [
    /\s/,
    $.comment,
  ],

  rules: {
    source_file: $ => repeat($._declaration),

    _declaration: $ => choice(
      $.entity_declaration,
      $.capability_declaration,
      $.contract_declaration,
      $.flow_declaration,
      $.boundary_declaration,
      $.interface_declaration,
      $.function_declaration
    ),

    // Comments
    comment: $ => token(choice(
      seq('//', /.*/),
      seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/')
    )),

    // Entity
    entity_declaration: $ => seq(
      'entity',
      field('name', $.identifier),
      '{',
      repeat(choice($.field_declaration, $.method_declaration)),
      '}'
    ),

    // Capability
    capability_declaration: $ => seq(
      'capability',
      field('name', $.identifier),
      '{',
      repeat($.field_declaration),
      '}'
    ),

    // Boundary
    boundary_declaration: $ => seq(
      'boundary',
      field('name', $.identifier),
      '{',
      repeat(choice($.grant_clause, $.quota_clause, $.bind_clause)),
      '}'
    ),

    grant_clause: $ => seq(
      'grant',
      field('capability', $.identifier),
      '{',
      commaSep($.named_argument),
      '}',
      ';'
    ),

    quota_clause: $ => seq(
      'quota',
      '{',
      repeat(seq(choice('memory_limit', 'fuel_limit'), ':', $.number, ';')),
      '}'
    ),

    bind_clause: $ => seq('bind', field('entity', $.identifier), ';'),

    // Interface
    interface_declaration: $ => seq(
      'interface',
      field('name', $.identifier),
      '{',
      repeat($.interface_method),
      '}'
    ),

    interface_method: $ => seq(
      'fn',
      field('name', $.identifier),
      '(', commaSep($.parameter), ')',
      optional(seq('->', field('return_type', $.type))),
      optional($.effect_clause),
      ';'
    ),

    // Contracts
    contract_declaration: $ => seq(
      'contract',
      field('name', $.identifier),
      '{',
      repeat($.contract_clause),
      '}'
    ),

    contract_clause: $ => seq(
      choice('requires', 'ensures', 'invariant'),
      $._expression,
      optional(';')
    ),

    // Flow
    flow_declaration: $ => seq(
      'flow',
      field('name', $.identifier),
      '(', commaSep($.parameter), ')',
      optional(seq('->', field('return_type', $.type))),
      optional($.effect_clause),
      $.block
    ),

    // Function & Methods
    function_declaration: $ => seq(
      optional('pub'),
      'fn',
      field('name', $.identifier),
      '(', commaSep($.parameter), ')',
      optional(seq('->', field('return_type', $.type))),
      optional($.effect_clause),
      repeat($.contract_clause),
      $.block
    ),

    method_declaration: $ => $.function_declaration,

    effect_clause: $ => seq('with', '[', commaSep($.identifier), ']'),

    field_declaration: $ => seq(
      choice('let', 'mut'),
      field('name', $.identifier),
      ':',
      field('type', $.type),
      optional(seq('=', $._expression)),
      ';'
    ),

    parameter: $ => seq(
      optional('mut'),
      field('name', $.identifier),
      ':',
      field('type', $.type)
    ),

    named_argument: $ => seq(
      field('name', $.identifier),
      ':',
      field('value', $._expression)
    ),

    // Types
    type: $ => choice(
      $.primitive_type,
      $.generic_type,
      $.identifier
    ),

    primitive_type: $ => choice(
      'Int', 'Float', 'Bool', 'String', 'Bytes', 'Unit'
    ),

    generic_type: $ => seq(
      $.identifier,
      '<',
      commaSep1($.type),
      '>'
    ),

    // Statements
    block: $ => seq(
      '{',
      repeat($._statement),
      '}'
    ),

    _statement: $ => choice(
      $.let_statement,
      $.return_statement,
      $.if_statement,
      $.expression_statement
    ),

    let_statement: $ => seq(
      'let',
      optional('mut'),
      field('name', $.identifier),
      optional(seq(':', field('type', $.type))),
      optional(seq('=', field('value', $._expression))),
      ';'
    ),

    return_statement: $ => seq(
      'return',
      optional($._expression),
      ';'
    ),

    if_statement: $ => seq(
      'if',
      field('condition', $._expression),
      field('consequence', $.block),
      optional(seq('else', field('alternative', choice($.block, $.if_statement))))
    ),

    expression_statement: $ => seq($._expression, ';'),

    // Expressions
    _expression: $ => choice(
      $.binary_expression,
      $.unary_expression,
      $.call_expression,
      $.field_access,
      $.identifier,
      $.number,
      $.string,
      $.boolean
    ),

    binary_expression: $ => prec.left(choice(
      prec(1, seq($._expression, choice('||', 'or'), $._expression)),
      prec(2, seq($._expression, choice('&&', 'and'), $._expression)),
      prec(3, seq($._expression, choice('==', '!='), $._expression)),
      prec(4, seq($._expression, choice('<', '<=', '>', '>='), $._expression)),
      prec(5, seq($._expression, choice('+', '-'), $._expression)),
      prec(6, seq($._expression, choice('*', '/', '%'), $._expression))
    )),

    unary_expression: $ => prec(7, seq(
      choice('!', '-', 'not'),
      $._expression
    )),

    call_expression: $ => prec(8, seq(
      field('function', choice($.identifier, $.field_access)),
      '(',
      commaSep($._expression),
      ')'
    )),

    field_access: $ => prec(9, seq(
      field('object', choice($.identifier, 'self')),
      '.',
      field('field', $.identifier)
    )),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,
    number: $ => /[0-9]+(\.[0-9]+)?/,
    string: $ => /"[^"\\]*(?:\\.[^"\\]*)*"/,
    boolean: $ => choice('true', 'false'),
  }
});

function commaSep(rule) {
  return optional(commaSep1(rule));
}

function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}
