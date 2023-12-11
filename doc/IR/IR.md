# IR
The intermediate representation is a step above assembly.
It supports concepts such as generalized syscall-like operations and allocations
The intermediate representation also has concepts of functions, variables and structs
IR only allows ascii. If you want to use utf-8 in constant strings you must define them in a header at the top of the file and reference them with identifiers

Comment lines begin with // or ;

## File header
The file header begins as such
```
--NIR--
^STRING1="HELLO WORLD";
^STRING2="😘";
--END--
```




## Variables and identifiers
Globally scoped identifiers such as functions are prefixed with `$`  
Variables are prefixed with `&`

## Registers


## Basic operations
Operation usage in this table loosely follows this syntax:  
`[ATOM]` - Use the raw value of this atom

---

Atoms can look like:  
`$[IDENT]` - Global identifier  
`&[IDENT]` - Local identifier  
`!([TYPE])[CONST]` - e.g. !(u32)11  
`cptroffset [TYPE] [CONST]` - similar to ptroffset instruction but represents a constant number to offset by.   
`sptroffset [TYPE] [PATH]` - gets a ptroffset to a struct field

---

|Op   |Usage  |Comment|
|---- |-------|-------|
|add  |`&x = add(s64) &something, !(s64)0x10`||
|sub  |`&y = sub(u64) &something, !(u64)0b1111`||
|div  |Same as above||
|mul  |Same as above||
|call |`&result = call(s64) $magik (s64 &x, u64 &y)`||
|ret |`ret(s64) &result`|Ret without a type and operand is only acceptable in a void function|
|stalloc |`&newarr = stalloc s32 times !(u8)4`|Allocate on stack the size of given type multiplied by the operand. Returns a pointer to the memory|
|ptroffset|`&elementtwo = ptroffset(s32) ptr &newarr !(u32)1`|Zero-indexed offset by type|
|load|`&elementone = load(s32) ptr &newarr`||
|store|`store(s32) ptr &elementtwo &result` ||
|dbg  |`dbg(s64) &result`|Dbg may be a no-op or it may output the operand in some  way, depending on the target platform|


## Struct definition
For a struct with unnamed (fields are accessed with 0-indexed names) fields
```
struct $[STRUCT_NAME]  {[TYPE], [TYPE], ...}
```
For a struct with named fields
```
struct $[STRUCT_NAME]  {[NAME]=[TYPE], [NAME]=[TYPE], ...}
```

## Function definition
```
fn $[FUNC_NAME]([RETURN_TYPE]) ([ARG_TYPE] [ARG_NAME]) {
    [INSTRUCTION]
    [INSTRUCTION]
    ...
}
```
Void functions omit the return type but not the brackets surrounding it

## Types
Primitives
|s8|u8|   |
|---|---|---|
|s16|u16||
|s32|u32|f32|
|s64|u64|f64|

Types can also references struct definitions.
You can make a type a pointer by postfixing it with *


## Example programs
```
; Add two 32-bit unsigned numbers

fn $add_numbers(u32) (u32 x, u32 y) {

  ; Add the two numbers
  &result = add(u32) &x, &y

  ; Return the result
  ret(u32) &result

}

; Main function
fn $main() () {

  ; Numbers to add
  &num1 = !(u32)9
  &num2 = !(u32)11

  ; Call the add function
  &sum = call $add_numbers(u32) (u32 &num1, u32 &num2)

  ; Output the result
  dbg(u32) &sum

}
```

```
; Array
fn $main() () {
    ; Allocate on stack an array of 25 integers
    &arr = stalloc u32 times !(u8)25
    ; Set the 3rd element to 16
    &ptr3 = ptroffset(u32) ptr &arr !(u8)2
    store(u32) ptr &ptr3 !(u32)16
}
```