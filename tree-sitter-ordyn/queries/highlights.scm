; Tree-sitter Highlights query for ORDYN

; Foundational Constructs & Keywords
[
  "entity"
  "capability"
  "contract"
  "flow"
  "boundary"
  "interface"
] @keyword.storage

[
  "fn"
  "pub"
  "let"
  "mut"
  "grant"
  "quota"
  "bind"
  "with"
] @keyword

[
  "if"
  "else"
  "return"
] @keyword.control

; Contracts
[
  "requires"
  "ensures"
  "invariant"
] @keyword.coroutine

; Types
(primitive_type) @type.builtin
(generic_type (identifier) @type)

; Functions & Methods
(function_declaration name: (identifier) @function)
(interface_method name: (identifier) @function.method)
(call_expression function: (identifier) @function.call)
(call_expression function: (field_access field: (identifier) @function.call))

; Declarations
(entity_declaration name: (identifier) @type)
(capability_declaration name: (identifier) @type)
(boundary_declaration name: (identifier) @type)
(interface_declaration name: (identifier) @type)

; Identifiers & Fields
(parameter name: (identifier) @variable.parameter)
(field_declaration name: (identifier) @variable.other.member)
(field_access field: (identifier) @variable.other.member)
(field_access object: "self" @variable.language.this)

; Literals
(number) @number
(string) @string
(boolean) @boolean

; Comments
(comment) @comment
