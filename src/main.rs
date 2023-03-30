use miden_stdlib::StdLibrary;
use miden_vm::{
    execute, execute_iter, prove, verify, Assembler, Kernel, MemAdviceProvider, Program,
    ProgramInfo, ProofOptions, StackInputs,
};

fn main() {
    // instantiate the assembler with the standard library
    let assembler = Assembler::default()
        .with_library(&StdLibrary::default())
        .map_err(|err| format!("Failed to load stdlib - {}", err))
        .unwrap()
        .with_debug_mode(true);

    // compile Miden assembly source code into a program
    // this program will add two numbers (3 and 5) on the stack
    // you can change this to any other Miden program
    // check out our examples
    let program = assembler
        .compile("begin push.3 push.5 add end")
        .map_err(|err| format!("Failed to compile program - {}", err))
        .unwrap();

    // we also create some program_info
    let program_info = ProgramInfo::new(program.clone().hash(), Kernel::default());

    // use an empty list as initial stack
    let stack_inputs = StackInputs::default();

    // instantiate an empty advice provider
    let mut advice_provider = MemAdviceProvider::default();

    // execute the program with no inputs
    let _trace = execute(&program, stack_inputs.clone(), &mut advice_provider).unwrap();

    // now, execute the same program in debug mode and iterate over VM states
    for vm_state in execute_iter(&program, stack_inputs, advice_provider) {
        match vm_state {
            Ok(vm_state) => println!("{:?}", vm_state),
            Err(_) => println!("something went terribly wrong!"),
        }
    }

    // let's execute it and generate a STARK proof
    let (outputs, proof) = prove(
        &program,
        StackInputs::default(),       // we won't provide any inputs
        MemAdviceProvider::default(), // we won't provide advice inputs
        ProofOptions::default(),      // we'll be using default options
    )
    .unwrap();

    // the output should be 8
    assert_eq!(Some(&8), outputs.stack().first());

    // let's verify program execution
    match verify(program_info, StackInputs::default(), outputs, proof) {
        Ok(_) => println!("Execution verified!"),
        Err(msg) => println!("Something went terribly wrong: {}", msg),
    }
}
