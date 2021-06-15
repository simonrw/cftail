# cftail

Tail for cloudformation stacks.

## Usage

The program requires the name of the stack you wish to tail. Optionally, a timestamp can be specified with the `--since`
argument, which also prints all messages since that time.

In its usual mode, the program waits for stack events and prints with the same colour scheme as the web console.

With the `--nested` flag, any nested stacks will also be included in the output.


```
cftail 0.3.0-dev.1
Simon Walker
Tail CloudFormation deployments

Watch a log of deployment events for CloudFormation stacks from your console.

USAGE:
    cftail [FLAGS] [OPTIONS] [stack-names]...

FLAGS:
    -h, --help       
            Prints help information

    -n, --nested     
            Also fetch nested stacks and their deploy status

    -V, --version    
            Prints version information


OPTIONS:
    -s, --since <since>    
            When to start fetching data from. This could be a timestamp, text string, or the words `today` or
            `yesterday`

ARGS:
    <stack-names>...    
            Name of the stacks to tail
```
