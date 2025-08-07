# Constraints

## Version Constraints

OS version: openEuler 22.03 LTS SP1 or SP2
Architecture: x86 or AArch64

## Application Constraints

Currently, user-mode patches support only Redis and Nginx.

Note:

1. Currently, each software needs to be adapted to process the LINE macro. Currently, only Redis and Nginx are adapted. Other software that is not adapted may cause the patch size to be too large. (Parameters will be introduced in the future to support user adaptation.)
2. Each user-mode live patch can contain only one ELF file. To fix multiple bugs, you can pass the patch files of multiple bug fixes to the patch making parameters to make a live patch for multiple bugs.

## Language Constraints

Theoretically, patches are compared at the object file level, which is irrelevant to the programming language. Currently, only the C and C++ languages are tested.

## Others

- Only 64-bit OSs are supported.
- Only the ELF format can be hot-patched. Interpreted languages are not supported.
- Only GCC and G++ compilers are supported.
- The compiler must support the `-gdwarf`, `-ffunction-sections`, and `-fdata-sections` parameters.
- The debug information must be in the DWARF format.
- Cross compilation is not supported.
- Source files that are in different paths but have the same file name, same global variables, and same functions cannot be recognized.
- Assembly code, including **.S** files and inline assembly code, cannot be modified.
- External symbols (dynamic library dependencies) cannot be added.
- Multiple patches cannot be applied to the same binary file.
- Mixed compilation of C and C++ is not supported.
- C++ exceptions cannot be modified.
- The `-g3` group section compilation option, specific compilation optimization options, and specific GCC plugins are not supported.
- ifunc cannot be added by using `__attribute__((ifunc("foo")))`.
- TLS variables cannot be added by using `__thread int foo`.
