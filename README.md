# Scaffolded Repo for zkHack in Lisbon 2023
We want you to hack on the Miden VM. We want to provide a good experience for you to create, prove and verify programs and their execution. 

This repo contains all building blocks that are needed. In `main.rs` you will see that there is a program being created, executed, proven and verified. 

You can run `cargo build` and `cargo run` to see the example working. 

For more info about the Miden VM and that particular example, read further.

As additional ressources you can use: 

- [Miden Playground (online Miden Assembly Compiler)](https://0xpolygonmiden.github.io/examples/) - *to get started and understand Miden Assembly*
- [Miden VM Repo](https://github.com/0xPolygonMiden/miden-vm) - *to have the full developer feature set ([CLI](https://0xpolygonmiden.github.io/miden-vm/intro/usage.html#cli-interface), [Miden Debugger](https://0xpolygonmiden.github.io/miden-vm/intro/usage.html#miden-debugger), [REPL](https://0xpolygonmiden.github.io/miden-vm/intro/usage.html#repl))*

___

# Miden VM
This crate aggregates all components of Miden VM in a single place. Specifically, it re-exports functionality from [processor](../processor/), [prover](../prover/), and [verifier](../verifier/) crates. Additionally, when compiled as an executable, this crate can be used via a [CLI interface](#cli-interface) to execute Miden VM programs and to verify correctness of their execution.

## Basic concepts
An in-depth description of Miden VM is available in the full Miden VM [documentation](https://0xpolygonmiden.github.io/miden-vm/). In this section we cover only the basics to make the included examples easier to understand.

### Writing programs
Our goal is to make Miden VM an easy compilation target for high-level blockchain-centric languages such as Solidity, Move, Sway, and others. We believe it is important to let people write programs in the languages of their choice. However, compilers to help with this have not been developed yet. Thus, for now, the primary way to write programs for Miden VM is to use [Miden assembly](../assembly).

Miden assembler compiles assembly source code in a [program MAST](https://0xpolygonmiden.github.io/miden-vm/design/programs.html), which is represented by a `Program` struct. It is possible to construct a `Program` struct manually, but we don't recommend this approach because it is tedious, error-prone, and requires an in-depth understanding of VM internals. All examples throughout these docs use assembly syntax.

#### Program hash
All Miden programs can be reduced to a single 32-byte value, called program hash. Once a `Program` object is constructed, you can access this hash via `Program::hash()` method. This hash value is used by a verifier when they verify program execution. This ensures that the verifier verifies execution of a specific program (e.g. a program which the prover had committed to previously). The methodology for computing program hash is described [here](https://0xpolygonmiden.github.io/miden-vm/design/programs.html#program-hash-computation).

### Inputs / outputs
Currently, there are 3 ways to get values onto the stack:

1. You can use `push` instruction to push values onto the stack. These values become a part of the program itself, and, therefore, cannot be changed between program executions. You can think of them as constants.
2. The stack can be initialized to some set of values at the beginning of the program. These inputs are public and must be shared with the verifier for them to verify a proof of the correct execution of a Miden program. The number of elements at the top of the stack which can receive an initial value is limited to 16.
3. The program may request nondeterministic advice inputs from the prover. These inputs are secret inputs. This means that the prover does not need to share them with the verifier. There are three types of advice inputs: (1) a single advice stack which can contain any number of elements; (2) a key-mapped element lists which can be pushed onto the advice stack; (3) a Merkle store, which is used to provide nondeterministic inputs for instructions which work with Merkle trees. There are no restrictions on the number of advice inputs a program can request.

The stack is provided to Miden VM via `StackInputs` struct. These are public inputs of the execution, and should also be provided to the verifier. The secret inputs of the program are provided via `AdviceProvider` instances. There is one in-memory advice provider that can be commonly used for operations that won't require persistence: `MemAdviceProvider`.

Values remaining on the stack after a program is executed can be returned as stack outputs. You can specify exactly how many values (from the top of the stack) should be returned. Currently, the maximum number of outputs is limited to 16.

Having only 16 elements to describe public inputs and outputs of a program may seem limiting, however, just 4 elements are sufficient to represent a root of a Merkle tree or a sequential hash of elements. Both of these can be expanded into an arbitrary number of values by supplying the actual values non-deterministically via the advice provider.

## Usage
Miden crate exposes several functions which can be used to execute programs, generate proofs of their correct execution, and verify the generated proofs. How to do this is explained below, but you can also take a look at working examples [here](examples) and find instructions for running them via CLI [here](#fibonacci-example).

### Executing programs
To execute a program on Miden VM, you can use either `execute()` or `execute_iter()` functions. Both of these functions take the same arguments:

* `program: &Program` - a reference to a Miden program to be executed.
* `stack_inputs: StackInputs` - a set of public inputs with which to execute the program.
* `advice_provider: AdviceProvider` - an instance of an advice provider that yields secret, non-deterministic inputs to the prover.

The `execute()` function returns a `Result<ExecutionTrace, ExecutionError>` which will contain the execution trace of the program if the execution was successful, or an error, if the execution failed. You can inspect the trace to get the final state of the VM out of it, but generally, this trace is intended to be used internally by the prover during proof generation process.

The `execute_iter()` function returns a `VmStateIterator` which can be used to iterate over the cycles of the executed program for debug purposes. In fact, when we execute a program using this function, a lot of the debug information is retained and we can get a precise picture of the VM's state at any cycle. Moreover, if the execution results in an error, the `VmStateIterator` can still be used to inspect VM states right up to the cycle at which the error occurred.

For example:
```rust
use miden::{Assembler, execute, execute_iter, MemAdviceProvider, StackInputs};

// instantiate the assembler
let assembler = Assembler::default();

// compile Miden assembly source code into a program
let program = assembler
        .compile("begin push.3 push.5 add end")
        .map_err(|err| format!("Failed to compile program - {}", err))
        .unwrap();

// use an empty list as initial stack
let stack_inputs = StackInputs::default();

// instantiate an empty advice provider
let mut advice_provider = MemAdviceProvider::default();

// execute the program with no inputs
let trace = execute(&program, stack_inputs.clone(), &mut advice_provider).unwrap();

// now, execute the same program in debug mode and iterate over VM states
for vm_state in execute_iter(&program, stack_inputs, advice_provider) {
    match vm_state {
        Ok(vm_state) => println!("{:?}", vm_state),
        Err(_) => println!("something went terribly wrong!"),
    }
}
```

### Proving program execution
To execute a program on Miden VM and generate a proof that the program was executed correctly, you can use the `prove()` function. This function takes the following arguments:

* `program: &Program` - a reference to a Miden program to be executed.
* `stack_inputs: StackInputs` - a set of public inputs with which to execute the program.
* `advice_provider: AdviceProvider` - an instance of an advice provider that yields secret, non-deterministic inputs to the prover.
* `options: ProofOptions` - config parameters for proof generation. The default options target 96-bit security level.

If the program is executed successfully, the function returns a tuple with 2 elements:

* `outputs: StackOutputs` - the outputs generated by the program.
* `proof: ExecutionProof` - proof of program execution. `ExecutionProof` can be easily serialized and deserialized using `to_bytes()` and `from_bytes()` functions respectively.

#### Proof generation example
Here is a simple example of proving the previous example:
```rust

// let's execute it and generate a STARK proof
let (outputs, proof) = prove(
    &program,
    StackInputs::default(),       // we won't provide any inputs
    MemAdviceProvider::default(), // we won't provide advice inputs
    ProofOptions::default(),     // we'll be using default options
)
.unwrap();

// the output should be 8
assert_eq!(Some(&8), outputs.stack().first());
```

### Verifying program execution
To verify program execution, you can use the `verify()` function. The function takes the following parameters:

* `program_info: ProgramInfo` - a structure containing the hash of the program to be verified (represented as a 32-byte digest), and the hashes of the Kernel procedures used to execute the program.
* `stack_inputs: StackInputs` - a list of the values with which the stack was initialized prior to the program's execution..
* `stack_outputs: StackOutputs` - a list of the values returned from the stack after the program completed execution.
* `proof: ExecutionProof` - the proof generated during program execution.

Stack inputs are expected to be ordered as if they would be pushed onto the stack one by one. Thus, their expected order on the stack will be the reverse of the order in which they are provided, and the last value in the `stack_inputs` is expected to be the value at the top of the stack.

Stack outputs are expected to be ordered as if they would be popped off the stack one by one. Thus, the value at the top of the stack is expected to be in the first position of the `stack_outputs`, and the order of the rest of the output elements will also match the order on the stack. This is the reverse of the order of the `stack_inputs`.

The function returns `Result<u32, VerificationError>` which will be `Ok(security_level)` if verification passes, or `Err(VerificationError)` if verification fails, with `VerificationError` describing the reason for the failure.

> If a program with the provided hash is executed against some secret inputs and the provided public inputs, it will produce the provided outputs.

Notice how the verifier needs to know only the hash of the program - not what the actual program was.

#### Proof verification example
Here is a simple example of verifying execution of the program from the previous example:
```rust,ignore

// let's verify program execution
match miden::verify(program.hash(), StackInputs::default(), &[8], proof) {
    Ok(_) => println!("Execution verified!"),
    Err(msg) => println!("Something went terribly wrong: {}", msg),
}
```

## License
This project is [MIT licensed](../LICENSE).
