# println Builtin Function

The `println` function is a builtin function in the Yuni language that prints values to the console.

## Features

- **Variadic**: Accepts any number of arguments (including zero)
- **Automatic String Conversion**: Converts all argument types to strings automatically
- **Space Separation**: Multiple arguments are separated by spaces
- **Type Support**: Handles:
  - String literals and variables
  - Integer types (all sizes)
  - Floating-point types (all sizes)
  - Boolean values (prints as "true" or "false")

## Usage Examples

```yuni
// Empty println prints a blank line
println();

// Single string argument
println("Hello, World!");

// Multiple arguments
println("The answer is", 42);

// Variables of different types
let name = "Alice";
let age = 25;
let height = 5.6;
let is_student = true;

println("Name:", name, "Age:", age, "Height:", height, "Student:", is_student);
// Output: Name: Alice Age: 25 Height: 5.6 Student: true
```

## Implementation Details

### Semantic Analysis
- The `println` function is recognized as a builtin in the semantic analyzer
- It bypasses normal function type checking since it's variadic
- All arguments are type-checked to ensure they're valid expressions

### Code Generation
- Each argument is converted to a string using type-specific conversion functions:
  - `yuni_i64_to_string` for integers
  - `yuni_f64_to_string` for floats
  - `yuni_bool_to_string` for booleans
  - String values are used directly
- Arguments are concatenated with spaces using `yuni_string_concat`
- The final string is passed to `yuni_println` runtime function

### Runtime Requirements
The following runtime functions must be provided:
- `yuni_println(str: *const u8)` - Prints a string and adds a newline
- `yuni_string_concat(a: *const u8, b: *const u8) -> *const u8` - Concatenates two strings
- `yuni_i64_to_string(val: i64) -> *const u8` - Converts integer to string
- `yuni_f64_to_string(val: f64) -> *const u8` - Converts float to string
- `yuni_bool_to_string(val: bool) -> *const u8` - Converts boolean to string