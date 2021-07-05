# `elfbin`

`elfbin` is a command line tool and Rust library for constructing ELF object
files that contain symbols which refer to the contents of arbitrary files
you provide. You can then link the resulting file into your program written
in some other language that can produce ELF object files, such as C.

This can be useful for embedded systems and other similar situations where
there's no filesystem available to load data from and so any required data
must be linked directly into the program.

## Usage

`elfbin` expects arguments of the form `symbolname=filename` to specify which
symbols to define in the object file and which files to read their contents
from.

For example:

```bash
elfbin -o data.o image=foo.png music=foo.mid
```

`elfbin` also has options to specify what kind of ELF file to create. You'll
generally need to match these settings with what the linker for your target
platform expects:

```
    --class <class>          ELF Class [default: ELF64]
    --encoding <encoding>    ELF Encoding [default: LSB]
    --flags <flags>          Machine-specific ELF flags [default: 0x00000000]
    --machine <machine>      Target machine [default: none]
    -o <out>                 Output filename
```

For example, to generate a file suitable for linking into a program for an
ARM-Cortex-M0 (ARMv6-M) microcontroller:

```bash
elfbin -o data.o --class=ELF32 --encoding=LSB --machine=arm --flags=0x05000000
```

You can then include the `data.o` file in your linker invocation, along with
all of the `.o` files that resulted from compiling your source code.

## Writing Header Files

`elfbin` has no built-in support for generating C header files to allow you
to access your object data from elsewhere in your program.

You can do so manually by selecting a suitable data type to represent the
data you've linked and declaring an `extern const` variable of that type.
If you have no special data type to use -- for example, if your data is in
a file format that you'll need to parse before you can use it -- then an
array of type `uint8_t` from `stdint.h` could be a reasonable choice:

```c
#include <stdint.h>

extern uint8_t image[];
extern uint8_t music[];
```

Alternatively, if you make sure that your input data is of a suitable shape
for the struct layout used by your compiler then you could declare the
data as having a struct type.

Note that the symbol names you declare when running `elfbin` refer directly
to the data itself, not to a pointer to the data. Therefore you typically
shouldn't declare your symbol as having a pointer type in your header file,
unless you've intentionally created a file containing memory addresses.
