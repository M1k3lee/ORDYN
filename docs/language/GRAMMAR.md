# ORDYN Formal Grammar (EBNF)

**Specification Baseline:** v0.1.0  
**Grammar Notation:** ISO/IEC 14977 Extended Backus-Naur Form (EBNF)  

---

## 1. Top-Level Declarations

```ebnf
Program             = { Declaration } ;

Declaration         = EntityDecl
                    | CapabilityDecl
                    | ContractDecl
                    | FlowDecl
                    | BoundaryDecl
                    | InterfaceDecl
                    | FunctionDecl
                    | StructDecl
                    | EnumDecl
                    | ImportDecl ;

ImportDecl          = "import" Identifier [ "as" Identifier ] ";" ;
```

---

## 2. The Six Foundational Constructs

```ebnf
EntityDecl          = "entity" Identifier [ Generics ] "{" { EntityMember } "}" ;
EntityMember        = [ "pub" ] ( FieldDecl | MethodDecl ) ;
FieldDecl           = [ "mut" ] Identifier ":" Type [ "=" Expression ] ";" ;
MethodDecl          = "fn" Identifier [ Generics ] "(" [ ParameterList ] ")" 
                      [ "->" Type ] [ EffectClause ] [ ContractClause ] Block ;

CapabilityDecl      = "capability" Identifier "{" { FieldDecl } "}" ;

ContractDecl        = "contract" Identifier [ Generics ] "{" { ContractClause } "}" ;
ContractClause      = RequiresClause | EnsuresClause | InvariantClause ;
RequiresClause      = "requires" Expression ";" ;
EnsuresClause       = "ensures" Expression ";" ;
InvariantClause     = "invariant" Expression ";" ;

FlowDecl            = "flow" Identifier [ Generics ] "(" [ ParameterList ] ")" 
                      [ "->" Type ] [ EffectClause ] "{" FlowBody "}" ;
FlowBody            = Expression { "->" Expression } ;

BoundaryDecl        = "boundary" Identifier "{" { BoundaryMember } "}" ;
BoundaryMember      = GrantClause | QuotaClause | EntityBinding ;
GrantClause         = "grant" Identifier "{" { Identifier ":" Expression "," } "}" ";" ;
QuotaClause         = "quota" "{" { Identifier ":" Expression ";" } "}" ;
EntityBinding       = "bind" Identifier ";" ;

InterfaceDecl       = "interface" Identifier [ Generics ] "{" { InterfaceMethod } "}" ;
InterfaceMethod     = "fn" Identifier [ Generics ] "(" [ ParameterList ] ")" 
                      [ "->" Type ] [ EffectClause ] ";" ;
```

---

## 3. Functions, Types, and Expressions

```ebnf
FunctionDecl        = [ "pub" ] "fn" Identifier [ Generics ] "(" [ ParameterList ] ")" 
                      [ "->" Type ] [ EffectClause ] [ ContractClause ] Block ;

ParameterList       = Parameter { "," Parameter } ;
Parameter           = [ "mut" ] Identifier ":" Type ;

EffectClause        = "with" "[" [ EffectList ] "]" ;
EffectList          = Identifier { "," Identifier } ;

Type                = PrimitiveType
                    | NamedType
                    | GenericType
                    | TupleType
                    | FunctionType ;

PrimitiveType       = "Int" | "Float" | "Bool" | "String" | "Bytes" | "Unit" ;
NamedType           = Identifier ;
GenericType         = Identifier "<" Type { "," Type } ">" ;
TupleType           = "(" [ Type { "," Type } ] ")" ;
FunctionType        = "fn" "(" [ Type { "," Type } ] ")" "->" Type ;

Block               = "{" { Statement } "}" ;

Statement           = LetStmt
                    | AssignmentStmt
                    | ReturnStmt
                    | IfStmt
                    | WhileStmt
                    | BreakStmt
                    | ContinueStmt
                    | ExpressionStmt ;

LetStmt             = ( "let" [ "mut" ] | "mut" ) Identifier [ ":" Type ] [ "=" Expression ] ";" ;
AssignmentStmt      = Identifier "=" Expression ";" ;
ReturnStmt          = "return" [ Expression ] ";" ;
WhileStmt           = "while" Expression Block ;
BreakStmt           = "break" ";" ;
ContinueStmt        = "continue" ";" ;
ExpressionStmt      = Expression ";" ;

IfStmt              = "if" Expression Block [ "else" ( Block | IfStmt ) ] ;
MatchStmt           = "match" Expression "{" { MatchArm } "}" ;
MatchArm            = Pattern "=>" ( Expression "," | Block ) ;

Expression          = LogicalOrExpr ;
LogicalOrExpr       = LogicalAndExpr { "||" LogicalAndExpr } ;
LogicalAndExpr      = EqualityExpr { "&&" EqualityExpr } ;
EqualityExpr        = RelationalExpr { ( "==" | "!=" ) RelationalExpr } ;
RelationalExpr      = AdditiveExpr { ( "<" | "<=" | ">" | ">=" ) AdditiveExpr } ;
AdditiveExpr        = MultiplicativeExpr { ( "+" | "-" ) MultiplicativeExpr } ;
MultiplicativeExpr  = UnaryExpr { ( "*" | "/" | "%" ) UnaryExpr } ;
UnaryExpr           = ( "-" | "!" ) UnaryExpr | PrimaryExpr ;

PrimaryExpr         = Literal
                    | Identifier
                    | CallExpr
                    | FieldAccessExpr
                    | MethodCallExpr
                    | "(" Expression ")"
                    | Block ;

CallExpr            = Identifier "(" [ ArgumentList ] ")" ;
MethodCallExpr      = PrimaryExpr "." Identifier "(" [ ArgumentList ] ")" ;
FieldAccessExpr     = PrimaryExpr "." Identifier ;
ArgumentList        = Expression { "," Expression } ;

Literal             = IntegerLiteral
                    | FloatLiteral
                    | StringLiteral
                    | BooleanLiteral
                    | "None" ;

BooleanLiteral      = "true" | "false" ;
```
