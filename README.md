# cftail

Tail for cloudformation stacks.

## Usage

The program requires the name of the stack you wish to tail. Optionally, a timestamp can be specified with the `--since`
argument, which also prints all messages since that time.

In its usual mode, the program waits for stack events and prints with the same colour scheme as the web console.


```
cftail 0.1.0

USAGE:
    cftail [OPTIONS] <stack-name>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -s, --since <since>

ARGS:
    <stack-name>
```

## Downloads

| Arch                 | Link                                                                                                                                   |
|----------------------|----------------------------------------------------------------------------------------------------------------------------------------|
| x86_64-unknown-linux | [Link](https://gitlab.com/srwalker101/cftail/-/jobs/artifacts/main/raw/target/x86_64-unknown-linux-gnu/release/cftail?job=build_linux) |
| x86_64-apple-darwin  | [Link](https://gitlab.com/srwalker101/cftail/-/jobs/artifacts/main/raw/target/x86_64-apple-darwin/release/cftail?job=build_macos)      |

vim: tw=0:nowrap
