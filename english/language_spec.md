# GoX Language Specification

This document defines the syntax and semantics of the **GoX** programming language.

GoX is a statically typed, Go-like language designed for compilation to LLVM, WASM, or a custom VM. It simplifies Go by removing complex features while adding an explicit interface/implements model.

---

## 1. Design Philosophy

### 1.1 Goals

- Familiar syntax for Go programmers
- Static, strong typing with local type inference
- Simple memory model: **object types** (heap-allocated, reference semantics) vs **value types** (copied on assignment)
- Explicit interface implementation via `implements` declarations
- Multiple backend targets (LLVM, WASM, VM)

### 1.2 Non-Goals

The following are explicitly out of scope:

- Generics
- Explicit pointer types (`*T`), address-of (`&x`)

---

## 2. Memory Model

### 2.1 Value Types vs Object Types

GoX distinguishes two categories of types:

| Category | Types | Assignment | Zero Value |
|----------|-------|------------|------------|
| **Value** | `int`, `float`, `bool`, `string`, `byte`, `[N]T` | Copies data | Type-specific |
| **Object** | `struct`, `interface`, `[]T`, `map[K]V`, `func(...)` | Copies reference | `nil` |

**Value type zero values**:
- `int` → `0`
- `float` → `0.0`
- `bool` → `false`
- `string` → `""`
- `byte` → `0`
- `[N]T` → each element is zero value of `T`

**Object type zero values**: Always `nil`.

### 2.2 Named Type Inheritance

When declaring `type T U`:
- `T` inherits the **category** (value or object) of `U`
- `T` inherits the **zero value** of `U`
- `T` inherits the **comparability** of `U`

```gox
type MyInt int;       // value type, zero = 0, comparable
type Users []User;    // object type, zero = nil
type Handler func();  // object type, zero = nil
```

### 2.3 Struct Reference Semantics

> **⚠️ Key Difference from Go**: In GoX, `struct` types are object types with reference semantics.

**Consequences**:

1. `var u User;` initializes `u` to `nil`, not a zero-valued struct
2. `u := User{};` creates a new object with zero-valued fields; `u != nil`
3. Assignment copies the reference: after `v := u;`, both `v` and `u` refer to the same object
4. Mutations through one variable are visible through the other
5. Field access (`u.name`) or method calls (`u.Method()`) on `nil` are **runtime errors**

```gox
var u User;           // u == nil
u.name = "x";         // RUNTIME ERROR: nil dereference

u = User{};           // u != nil, new object created
u.name = "Alice";     // OK

v := u;               // v and u refer to same object
v.name = "Bob";       // u.name is now also "Bob"
```

### 2.4 The `nil` Literal

`nil` represents the absence of an object for object types.

**Static Rules**:
- `nil` may be assigned to any object type
- `nil` cannot be assigned to value types (compile error)
- `var x = nil;` is invalid: type cannot be inferred
- `x := nil;` is invalid: type cannot be inferred (see §5.3)

**Runtime Rules**:
- Field access on `nil` → runtime error
- Method call on `nil` → runtime error
- Index access on `nil` slice/map → runtime error

### 2.5 Comparability Rules

Types are classified as **comparable** or **non-comparable**:

| Type | Comparable | Comparison Semantics |
|------|------------|---------------------|
| `int`, `float`, `byte` | ✅ | Value equality |
| `bool` | ✅ | Value equality |
| `string` | ✅ | Content equality |
| `[N]T` (if `T` comparable) | ✅ | Element-wise equality |
| Named value type | ✅ (inherits) | Per underlying type |
| `struct` | ❌ | Only `== nil` / `!= nil` |
| `interface` | ❌ | Only `== nil` / `!= nil` |
| `[]T` | ❌ | Only `== nil` / `!= nil` |
| `map[K]V` | ❌ | Only `== nil` / `!= nil` |
| `func(...)` | ❌ | Only `== nil` / `!= nil` |

**Rules**:
- `==` and `!=` require both operands to be comparable, OR one operand to be `nil` and the other an object type
- `<`, `<=`, `>`, `>=` are only valid for numeric types (`int`, `float`, `byte`) and `string`

```gox
1 == 2              // OK: int comparable
"a" < "b"           // OK: string ordered
u == nil            // OK: struct vs nil
u == v              // ERROR: struct not comparable (except nil)
s == nil            // OK: slice vs nil
s == t              // ERROR: slices not comparable
```

### 2.6 Parameter Passing

All parameters are passed by value. For object types, the "value" is a reference, so mutations inside a function affect the caller's object.

---

## 3. Lexical Structure

### 3.1 Identifiers

```ebnf
Ident  ::= Letter { Letter | Digit | "_" } ;
Letter ::= "A".."Z" | "a".."z" ;
Digit  ::= "0".."9" ;
```

Identifiers are case-sensitive.

### 3.2 Keywords

The following are reserved keywords:

```
break     case      chan      const     continue
default   defer     else      fallthrough false
for       func      go        goto      if
implements import   interface map       nil
package   range     return    select    struct
switch    true      type      var
```

> **Note**: `panic` and `recover` are **built-in functions**, not keywords. See §10.

### 3.3 Predefined Identifiers

The following are predeclared but can be shadowed:

```
// Types
int  float  bool  string  byte

// Constants
iota

// Zero value
nil

// Blank identifier
_

// Functions (compiler built-ins)
len  cap  append  make  close  panic  recover  print  println
```

**Blank identifier `_`**: Used to discard values or declare unused variables. It can appear on the left side of assignments and short variable declarations.

```gox
_, err := doSomething();   // discard first return value
for _, v := range slice {  // discard index
    println(v);
}
```

### 3.4 Operators and Punctuation

```
+    -    *    /    %
==   !=   <    <=   >    >=
&&   ||   !
<-                              // channel send/receive
=    :=   +=   -=   *=   /=   %=
(    )    [    ]    {    }
,    :    ;    .    ...
```

### 3.5 Literals

```ebnf
IntLit    ::= Digit+ ;
FloatLit  ::= Digit+ "." Digit+ ;
StringLit ::= '"' { char | escape } '"' ;
BoolLit   ::= "true" | "false" ;
NilLit    ::= "nil" ;
```

Escape sequences: `\n`, `\t`, `\\`, `\"`.

> **Note**: GoX only supports double-quoted strings. There are no raw string literals (backticks).

### 3.6 Semicolons

Semicolons terminate statements and declarations. The lexer automatically inserts a semicolon after a line's final token if that token is:
- An identifier, literal, or keyword (`break`, `continue`, `return`, `true`, `false`, `nil`)
- A closing delimiter: `)`, `]`, `}`

### 3.7 Comments

```
// Single-line comment
/* Multi-line
   comment */
```

---

## 4. Program Structure

### 4.1 Source Files

```ebnf
File ::= PackageClause? ImportDecl* TopDecl* ;

PackageClause ::= "package" Ident ";" ;
ImportDecl    ::= "import" StringLit ";" ;
```

### 4.2 Top-Level Declarations

```ebnf
TopDecl ::= VarDecl
          | ConstDecl
          | TypeDecl
          | InterfaceDecl
          | ImplementsDecl
          | FuncDecl ;
```

---

## 5. Declarations

### 5.1 Variables

```ebnf
VarDecl     ::= "var" ( VarSpec ";" | "(" VarSpecList ")" ";" ) ;
VarSpecList ::= ( VarSpec ";" )* ;
VarSpec     ::= IdentList Type? ( "=" ExprList )? ;
```

**Grouped declarations** use parentheses:
```gox
var (
    x int;
    y = 42;
    a, b = 1, 2;
);
```

**Static Rules**:
- If `Type` is omitted, `Expr` is required and type is inferred
- If `Expr` is omitted, variable is initialized to zero value (which is `nil` for object types)
- If `Expr` is `nil`, `Type` is required

```gox
var x int;           // x = 0
var y = 42;          // y inferred as int
var u User;          // u = nil (User is struct, an object type)
var h Handler = nil; // OK: type is explicit
var z = nil;         // ERROR: cannot infer type
```

### 5.2 Constants

```ebnf
ConstDecl     ::= "const" ( ConstSpec ";" | "(" ConstSpecList ")" ";" ) ;
ConstSpecList ::= ( ConstSpec ";" )* ;
ConstSpec     ::= IdentList Type? ( "=" ExprList )? ;
```

Constants require initializers (except when using `iota` continuation). `nil` is not a valid constant value.

**Grouped declarations** use parentheses:
```gox
const (
    Pi = 3.14159;
    E  = 2.71828;
);
```

**iota**: A predeclared identifier representing successive untyped integer constants. It resets to 0 at each `const` block and increments by 1 for each ConstSpec.

```gox
const (
    Sunday = iota;   // 0
    Monday;          // 1 (implicit = iota)
    Tuesday;         // 2
);

const (
    _  = iota;             // 0, ignored
    KB = 1 << (10 * iota); // 1 << 10 = 1024
    MB;                    // 1 << 20
    GB;                    // 1 << 30
);
```

```gox
const Pi = 3.14159;
const MaxSize int = 1024;
const Empty = nil;   // ERROR: nil cannot be const
```

### 5.3 Short Variable Declaration

```ebnf
ShortVarDecl ::= IdentList ":=" ExprList ";" ;
IdentList    ::= Ident ( "," Ident )* ;
ExprList     ::= Expr ( "," Expr )* ;
```

**Static Rules**:
- Only valid inside blocks (not at package level)
- **Always declares new variables** (shadowing is allowed)
- Number of identifiers must equal number of expressions
- Type of each variable = static type of corresponding expression
- If expression is `nil` with no inferable context type → compile error

> **⚠️ Difference from Go**: In Go, `:=` may reuse an existing variable if at least one new variable is declared. In GoX, `:=` **always** declares new variables. It never degrades to assignment.

```gox
x := 10;
x := 20;     // OK: shadows outer x, declares new x
x := nil;    // ERROR: cannot infer type from nil
```

### 5.4 Type Declarations

```ebnf
TypeDecl ::= "type" Ident Type ";" ;
```

The new type inherits the category (value/object), zero value, and comparability from the underlying type (see §2.2).

```gox
type User struct {
    name string;
    age  int "json:\"name\"";
};
```

> **Note**: Struct field tags are metadata strings. Their semantics are defined by tooling/standard library (e.g., JSON serialization). The compiler preserves them in the AST but does not interpret them.

---

## 6. Types

### 6.1 Type Grammar

```ebnf
Type ::= Ident
       | ArrayType
       | SliceType
       | MapType
       | ChanType
       | FuncType
       | StructType ;
```

> **Note**: Interface types cannot appear inline. Use named interface types declared via `InterfaceDecl`.

### 6.2 Built-in Types

| Type | Category | Description |
|------|----------|-------------|
| `int` | Value | Platform-sized signed integer |
| `float` | Value | Platform-sized floating point |
| `bool` | Value | Boolean (`true`/`false`) |
| `string` | Value | Immutable UTF-8 string |
| `byte` | Value | Alias for 8-bit unsigned integer |

### 6.3 Arrays

```ebnf
ArrayType ::= "[" IntLit "]" Type ;
```

Arrays are value types with fixed length.

```gox
var a [4]int;  // [0, 0, 0, 0]
```

### 6.4 Slices

```ebnf
SliceType ::= "[" "]" Type ;
```

Slices are object types referencing a dynamic sequence.

```gox
var s []int;         // s == nil
s = []int{1, 2, 3};  // s != nil
```

### 6.5 Maps

```ebnf
MapType ::= "map" "[" Type "]" Type ;
```

Maps are object types providing key-value storage.

**Key type restriction**: The key type must be a **comparable value type**:

| Valid Keys | Invalid Keys |
|------------|--------------|
| `int`, `float`, `byte` | `struct` |
| `bool` | `interface` |
| `string` | `[]T` (slice) |
| `[N]T` (array of comparable) | `map[K]V` |
| Named value types | `func(...)` |

```gox
var m map[string]int;      // m == nil
m = map[string]int{};      // m != nil, empty map
m["key"] = 42;

var bad map[User]int;      // ERROR: User (struct) not a valid key type
```

### 6.6 Channels

```ebnf
ChanType ::= "chan" Type ;
```

Channels are object types used for communication between goroutines. Zero value is `nil`.

> **Note**: GoX only supports bidirectional channels. Unlike Go, there are no directional channel types (`<-chan T` or `chan<- T`).

```gox
var ch chan int;           // ch == nil
ch = make(chan int);       // unbuffered channel
ch = make(chan int, 10);   // buffered channel with capacity 10
```

**Operations**:
- `ch <- value` — send value to channel (blocks if unbuffered and no receiver)
- `value := <-ch` — receive from channel (blocks if no value available)
- `value, ok := <-ch` — receive with closed check (`ok` is `false` if channel closed and empty)

```gox
ch <- 42;           // send
x := <-ch;          // receive
close(ch);          // close channel (no more sends allowed)
```

### 6.7 Functions

```ebnf
FuncType      ::= "func" "(" ParamTypeList? ")" ResultType? ;
ParamTypeList ::= Type ( "," Type )* ;
ResultType    ::= Type | "(" Type ( "," Type )* ")" ;
```

Function types are object types. Zero value is `nil`.

```gox
var f func(int) int;  // f == nil
```

### 6.8 Variadic Functions

```ebnf
VariadicParam ::= Ident "..." Type ;
```

A function may have a variadic final parameter. The caller may pass zero or more arguments of that type.

```gox
func sum(nums ...int) int {
    total := 0;
    for i := 0; i < len(nums); i += 1 {
        total += nums[i];
    }
    return total;
}

sum(1, 2, 3);        // nums = []int{1, 2, 3}
sum();               // nums = []int{}

s := []int{1, 2, 3};
sum(s...);           // spread slice as arguments
```

### 6.9 Structs

```ebnf
StructType ::= "struct" "{" FieldDecl* "}" ;
FieldDecl  ::= ( IdentList Type | Type ) Tag? ";" ;
Tag        ::= StringLit ;
```

Structs are **object types** (see §2.3). Fields may have optional tags for metadata.

**Anonymous fields** (embedding): A field can be just a type name, creating an embedded field. The type must be a named type or pointer to named type.

```gox
type User struct {
    id   int    "json:\"id\"";
    name string "json:\"name\"";
};

type Employee struct {
    User;           // anonymous/embedded field
    department string;
};

// Access embedded fields directly:
e := Employee{};
e.name = "Alice";   // accesses User.name
```

> **Note**: Tags use double-quoted strings with escaped inner quotes.

---

## 7. Interfaces and Methods

### 7.1 Interface Declarations

```ebnf
InterfaceDecl ::= "interface" Ident "{" InterfaceElem* "}" ";" ;
InterfaceElem ::= MethodSpec | EmbeddedIface ;
MethodSpec    ::= Ident "(" ParamList? ")" ResultType? ";" ;
EmbeddedIface ::= Ident ";" ;
ParamList     ::= Param ( "," Param )* ;
Param         ::= IdentList Type ;   // Type sharing: x, y int
```

```gox
interface Reader {
    Read(buf []byte) (int, Error);
};

interface ReadWriter {
    Reader;  // embedding
    Write(buf []byte) (int, Error);
};
```

> **Scope Limitation**: Embedded interfaces must be unqualified names from the current package. To embed an external interface like `io.Reader`, first create a local alias: `type Reader = io.Reader;`.

### 7.2 Interface Method Sets

The **method set** of an interface is computed as follows:

1. Start with directly declared methods
2. For each embedded interface `I`, recursively compute its method set
3. Union all method sets
4. If two methods have the same name:
   - If signatures are identical → collapse to one method
   - If signatures differ → compile error

```gox
interface A { Foo() int; };
interface B { Foo() int; Bar(); };
interface C { A; B; };  // method set = {Foo() int, Bar()}

interface D { Foo() string; };
interface E { A; D; };  // ERROR: Foo has conflicting signatures
```

### 7.3 Type Method Sets

The **method set** of a named type `T` is the set of all methods declared with receiver type `T`.

```gox
type User struct { ... };

func (u User) Name() string { ... }
func (u User) SetName(n string) { ... }

// Method set of User = {Name() string, SetName(string)}
```

Only **named types** can have methods. You cannot define methods on built-in types, arrays, slices, maps, or function types directly.

### 7.4 Method Declarations

```ebnf
FuncDecl ::= "func" Receiver? Ident "(" ParamList? ")" ResultType? Block ;
Receiver ::= "(" Ident Ident ")" ;
```

The receiver consists of a name and a **named type**. Anonymous types (arrays, slices, maps, func) are not allowed as receivers.

```gox
func (u User) Name() string {
    return u.name;
}

func (u User) SetName(name string) {
    u.name = name;  // modifies the underlying object
}

func (a [4]int) Sum() int { ... }  // ERROR: receiver must be named type
```

**Runtime**: Calling a method on a `nil` receiver is a runtime error.

### 7.5 Implements Declarations

```ebnf
ImplementsDecl ::= "implements" Ident ":" IdentList ";" ;
```

Explicitly declares that a type implements one or more interfaces.

```gox
implements User : Reader, Writer;
```

**Static Verification**:
1. Compute the flattened method set of each interface (per §7.2)
2. Compute the method set of the type (per §7.3)
3. The type's method set must include all required methods
4. Each method signature must match exactly (name, parameter types, result types)
5. Missing or mismatched methods → compile error

---

## 8. Statements

### 8.1 Statement Grammar

```ebnf
Stmt ::= Block
       | VarDecl
       | ConstDecl
       | ShortVarDecl
       | Assignment
       | ExprStmt
       | ReturnStmt
       | IfStmt
       | ForStmt
       | SwitchStmt
       | SelectStmt
       | GoStmt
       | DeferStmt
       | SendStmt
       | BreakStmt
       | ContinueStmt
       | GotoStmt
       | FallthroughStmt
       | LabeledStmt
       | EmptyStmt ;

EmptyStmt ::= ";" ;
ExprStmt  ::= Expr ";" ;
```

### 8.2 Blocks

```ebnf
Block ::= "{" Stmt* "}" ;
```

Blocks introduce lexical scope.

### 8.3 Assignments

```ebnf
Assignment ::= ExprList AssignOp ExprList ";" ;
AssignOp   ::= "=" | "+=" | "-=" | "*=" | "/=" | "%=" ;
ExprList   ::= Expr ( "," Expr )* ;
```

**Evaluation Order** (for multi-assignment `a, b = x, y`):
1. All RHS expressions are evaluated left-to-right
2. Results are stored in temporaries
3. All LHS locations are assigned left-to-right from temporaries

This guarantees that `a, b = b, a` swaps the values (or references, for object types).

```gox
x = 10;
a, b = b, a;  // swap: evaluates b, a first, then assigns
count += 1;
```

### 8.4 Return

```ebnf
ReturnStmt ::= "return" ExprList? ";" ;
```

**Static Rules**:

| Function Returns | `return` Form | Validity |
|------------------|---------------|----------|
| Nothing (no ResultType) | `return;` | ✅ Required |
| Nothing | `return expr;` | ❌ Error |
| Single type `T` | `return;` | ❌ Error |
| Single type `T` | `return expr;` | ✅ Required, `expr` must be type `T` |
| Multiple types `(T1, T2, ...)` | `return;` | ❌ Error |
| Multiple types | `return e1, e2, ...;` | ✅ Count and types must match |

```gox
func f() { return; }              // OK
func g() int { return 42; }       // OK
func h() (int, string) { return 1, "x"; }  // OK
func i() int { return; }          // ERROR
func j() { return 1; }            // ERROR
```

### 8.5 If

```ebnf
IfStmt     ::= "if" SimpleStmt? ";"? Expr Block ( "else" ( IfStmt | Block ) )? ;
SimpleStmt ::= ExprStmt | Assignment | ShortVarDecl ;
```

Optional init statement before condition. Condition must be type `bool`.

```gox
if x > 0 {
    // ...
} else if x < 0 {
    // ...
} else {
    // ...
}

// With init statement:
if x := compute(); x > 0 {
    println(x);
}

if err := doSomething(); err != nil {
    return err;
}
```

### 8.6 For

```ebnf
ForStmt        ::= "for" ForClause Block ;
ForClause      ::= Expr | ForThreeClause | ForRangeClause ;
ForThreeClause ::= SimpleStmt? ";" Expr? ";" SimpleStmt? ;
ForRangeClause ::= ( IdentList ( ":=" | "=" ) )? "range" Expr ;
SimpleStmt     ::= ExprStmt | Assignment | ShortVarDecl ;
```

**Parsing Note**: The parser distinguishes forms by the presence of `;` or `range` keyword.

```gox
for x < 10 { ... }                    // while-style
for i := 0; i < 10; i += 1 { ... }    // C-style
for ; ; { ... }                       // infinite
for i, v := range slice { ... }      // range over slice
for k, v := range m { ... }          // range over map
for i := range slice { ... }         // range with index only
for range ch { ... }                  // range with no variables
```

**Range Semantics**:
- For slices/arrays: `i` is index (int), `v` is element value
- For maps: `k` is key, `v` is value
- For strings: `i` is byte index, `v` is rune (int)
- For channels: `v` is received value (single variable only)

### 8.7 Switch

```ebnf
SwitchStmt    ::= "switch" ( SimpleStmt ";" )? Expr? "{" CaseClause* DefaultClause? "}" ;
CaseClause    ::= "case" ExprList ":" Stmt* ;
DefaultClause ::= "default" ":" Stmt* ;
```

Optional init statement and optional tag expression. If no tag expression, cases are evaluated as boolean conditions.

Cases implicitly break unless `fallthrough` is used.

```gox
switch x {
case 1:
    println("one");
case 2, 3:
    println("two or three");
default:
    println("other");
}

// With init statement:
switch x := getValue(); x {
case 1:
    println("one");
}

// Without tag (boolean cases):
switch {
case x > 0:
    println("positive");
case x < 0:
    println("negative");
default:
    println("zero");
}
```

**Type Rule**: If any case expression is `nil`, the switch expression must have an object type.

```gox
switch handler {
case nil:
    // handler is nil
default:
    handler.Serve();
}
```

### 8.8 Select

```ebnf
SelectStmt   ::= "select" "{" SelectCase* "}" ;
SelectCase   ::= ( "case" ( SendStmt | RecvStmt ) | "default" ) ":" Stmt* ;
RecvStmt     ::= ( IdentList ( ":=" | "=" ) )? "<-" Expr ;
```

`select` waits on multiple channel operations. One ready case is chosen at random. If no case is ready and there's a `default`, it executes immediately.

```gox
select {
case msg := <-ch1:
    println("received", msg);
case ch2 <- value:
    println("sent");
default:
    println("no communication ready");
}
```

### 8.9 Go

```ebnf
GoStmt ::= "go" Expr ";" ;
```

Starts a new goroutine executing the function call. The expression must be a function or method call.

```gox
go handleRequest(conn);
go func() {
    // anonymous function
}();
```

### 8.10 Defer

```ebnf
DeferStmt ::= "defer" Expr ";" ;
```

Defers execution of a function call until the surrounding function returns. Arguments are evaluated immediately, but the call is deferred. Multiple defers execute in LIFO order.

```gox
func readFile(path string) {
    f := open(path);
    defer close(f);       // called when function returns
    // ... use f ...
}
```

### 8.11 Send

```ebnf
SendStmt ::= Expr "<-" Expr ";" ;
```

Sends a value to a channel. The first expression must be a channel type, the second is the value to send.

```gox
ch <- 42;
```

### 8.12 Goto and Labels

```ebnf
GotoStmt    ::= "goto" Ident ";" ;
LabeledStmt ::= Ident ":" Stmt ;
```

`goto` transfers control to the labeled statement. Labels are scoped to the function body.

```gox
func example() {
    if condition {
        goto cleanup;
    }
    // ... normal code ...
cleanup:
    // cleanup code
}
```

### 8.13 Fallthrough

```ebnf
FallthroughStmt ::= "fallthrough" ";" ;
```

In a `switch` case, `fallthrough` transfers control to the first statement of the next case. It must be the last statement in a case.

```gox
switch x {
case 1:
    println("one");
    fallthrough;
case 2:
    println("one or two");
}
```

### 8.14 Break and Continue

```ebnf
BreakStmt    ::= "break" Ident? ";" ;
ContinueStmt ::= "continue" Ident? ";" ;
```

`break` exits innermost `for`, `switch`, or `select`. `continue` advances to next `for` iteration. Optional label targets a specific enclosing statement.

```gox
outer:
for i := 0; i < 10; i += 1 {
    for j := 0; j < 10; j += 1 {
        if condition {
            break outer;   // breaks outer loop
        }
    }
}
```

---

## 9. Expressions

### 9.1 Expression Grammar

```ebnf
Expr      ::= OrExpr ;
OrExpr    ::= AndExpr ( "||" AndExpr )* ;
AndExpr   ::= EqExpr ( "&&" EqExpr )* ;
EqExpr    ::= RelExpr ( ( "==" | "!=" ) RelExpr )* ;
RelExpr   ::= AddExpr ( ( "<" | "<=" | ">" | ">=" ) AddExpr )* ;
AddExpr   ::= MulExpr ( ( "+" | "-" ) MulExpr )* ;
MulExpr   ::= UnaryExpr ( ( "*" | "/" | "%" ) UnaryExpr )* ;
UnaryExpr ::= ( "+" | "-" | "!" ) UnaryExpr | Primary ;
```

### 9.2 Primary Expressions

```ebnf
Primary ::= Operand ( Selector | Index | Call | TypeAssertion )* ;
Operand ::= Ident | Literal | "(" Expr ")" | CompositeLit | Conversion | FuncLit ;
Literal ::= IntLit | FloatLit | StringLit | BoolLit | NilLit ;
FuncLit ::= "func" "(" ParamList? ")" ResultType? Block ;
```

### 9.3 Selectors, Indexing, and Slicing

```ebnf
Selector    ::= "." Ident ;
Index       ::= "[" Expr "]" ;
SliceExpr   ::= "[" Expr? ":" Expr? "]" ;
Call        ::= "(" ( Expr ( "," Expr )* "..."? )? ")" ;
```

**Slice Expressions**: Create a sub-slice from an array or slice.
- `a[low:high]` — elements from `low` to `high-1` (inclusive)
- `a[:high]` — elements from start to `high-1`
- `a[low:]` — elements from `low` to end
- `a[:]` — copy of entire slice

```gox
user.name       // field access (runtime error if user is nil)
arr[i]          // array/slice index
m["key"]        // map access
f(x, y)         // function call
s[1:3]          // slice expression
s[:len(s)-1]    // slice from start
```

### 9.4 Composite Literals

```ebnf
CompositeLit ::= Type "{" ElementList? "}" ;
ElementList  ::= Element ( "," Element )* ;
Element      ::= ( Key ":" )? Value ;
Key          ::= Ident | Expr ;
Value        ::= Expr | "{" ElementList? "}" ;
```

**Semantics**:
- For `struct`: `Key` must be a field name (`Ident`)
- For `map`: `Key` is an expression of the key type
- For `array`/`slice`: `Key` is an optional integer index

**Nested literal type elision**: For nested composite literals, the inner type can be omitted when it can be inferred from the outer type:

```gox
u := User{name: "Alice", age: 30};     // struct
a := [3]int{1, 2, 3};                  // array
s := []int{10, 20};                    // slice
m := map[string]int{"a": 1, "b": 2};   // map (key is Expr)

// Nested with type elision:
matrix := [][]int{{1, 2}, {3, 4}};     // inner []int elided
points := []Point{{1, 2}, {3, 4}};     // inner Point elided
```

### 9.5 Type Conversions

```ebnf
Conversion ::= Type "(" Expr ")" ;
```

```gox
i := int(f);
s := string(65);  // implementation-defined
```

### 9.6 Type Assertions

```ebnf
TypeAssertion ::= Expr "." "(" Type ")" ;
```

A type assertion extracts the concrete value from an interface value.

```gox
var i interface{} = "hello";

s := i.(string);       // panics if i is not a string
s, ok := i.(string);   // ok is false if i is not a string, no panic
```

**Rules**:
- The expression must be of interface type
- If the assertion fails and no `ok` variable is used, a runtime panic occurs
- With the `ok` form, the zero value is returned and `ok` is `false`

### 9.7 Receive Expression

```ebnf
RecvExpr ::= "<-" Expr ;
```

Receives a value from a channel. Can be used in expressions or statements.

```gox
x := <-ch;            // receive value
x, ok := <-ch;        // receive with closed check
```

---

## 10. Built-in Functions

The following functions are **compiler built-ins**. Their signatures use meta-notation and do not imply language-level generics or variadic support.

| Function | Behavior |
|----------|----------|
| `len(s)` | Returns length of slice, map, string, array, or channel |
| `cap(s)` | Returns capacity of slice or channel |
| `append(s, elems...)` | Returns new slice with elements appended |
| `make(T, size?, cap?)` | Allocates and initializes slice, map, or channel |
| `close(ch)` | Closes a channel (no more sends allowed) |
| `panic(v)` | Stops normal execution and begins panicking |
| `recover()` | Captures a panic value during deferred function execution |
| `print(args...)` | Debug output (no newline) |
| `println(args...)` | Debug output (with newline) |

**Usage Examples**:
```gox
s := make([]int, 10);      // slice of length 10
m := make(map[string]int); // empty map
s = append(s, 42);         // returns new slice
n := len(s);               // length
```

### 10.1 Panic and Recover

`panic` stops normal execution and begins unwinding the stack. Deferred functions still execute. `recover` can capture the panic value if called within a deferred function.

```gox
func safeCall(f func()) (err string) {
    defer func() {
        if r := recover(); r != nil {
            err = "caught panic";
        }
    }();
    f();
    return "";
}

func dangerous() {
    panic("something went wrong");
}

result := safeCall(dangerous);  // result = "caught panic"
```

### 10.2 Make

`make` creates and initializes slices, maps, and channels:

```gox
s := make([]int, 10);         // slice with length 10, capacity 10
s := make([]int, 10, 20);     // slice with length 10, capacity 20
m := make(map[string]int);    // empty map
ch := make(chan int);         // unbuffered channel
ch := make(chan int, 10);     // buffered channel with capacity 10
```

---

## 11. Type Switches

```ebnf
TypeSwitchStmt ::= "switch" ( Ident ":=" )? Expr "." "(" "type" ")" "{" TypeCaseClause* "}" ;
TypeCaseClause ::= "case" TypeList ":" Stmt* | "default" ":" Stmt* ;
TypeList       ::= Type ( "," Type )* ;
```

A type switch compares the dynamic type of an interface value against multiple types.

```gox
func describe(i interface{}) {
    switch v := i.(type) {
    case int:
        println("int:", v);
    case string:
        println("string:", v);
    case nil:
        println("nil");
    default:
        println("unknown type");
    }
}
```

**Rules**:
- The expression must be of interface type
- `v` is bound to the asserted type in each case
- Multiple types can be listed: `case int, float:`
- `default` handles unmatched types

---

## 12. Example Program

```gox
package main;

import "std/io";

type Error struct {
    msg string "json:\"message\"";
};

interface Logger {
    Log(msg string);
};

type ConsoleLogger struct {
    prefix string;
};

func (l ConsoleLogger) Log(msg string) {
    io.Println(l.prefix + ": " + msg);
}

implements ConsoleLogger : Logger;

func main() int {
    var logger Logger;  // logger == nil

    if logger == nil {
        logger = ConsoleLogger{prefix: "[APP]"};
    }

    logger.Log("Hello, GoX!");

    numbers := []int{1, 2, 3};
    numbers = append(numbers, 4);

    for i := 0; i < len(numbers); i += 1 {
        println(numbers[i]);
    }

    return 0;
}
```

