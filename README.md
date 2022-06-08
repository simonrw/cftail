# cftail

Tail for cloudformation stacks.

## Installation

- macos: `brew install mindriot101/cftail/cftail`
- source: `cargo install --git https://github.com/mindriot101/cftail`

## Usage

The program requires the name of the stack you wish to tail. Optionally, a timestamp can be specified with the `--since`
argument, which also prints all messages since that time. The format of this argument can be the following:

- a relative offset such as "2m" for two minutes (see the [documentation for the humantime](https://docs.rs/humantime/latest/humantime/fn.parse_duration.html) package).
- a full RFC3339 format datetime with timezone
- an RFC3339 datetime without timezone (assuming UTC)
- a unix timestamp
- either "today" or "yesterday"

In its usual mode, the program waits for stack events and prints with the same colour scheme as the web console.

With the `--nested` flag, any nested stacks will also be included in the output.

```
cftail 0.7.0
Simon Walker
Tail CloudFormation deployments

Watch a log of deployment events for CloudFormation stacks from your console.

USAGE:
    cftail [FLAGS] [OPTIONS] [stack-names]...

FLAGS:
    -h, --help                     Prints help information
    -n, --nested                   Also fetch nested stacks and their deploy status
        --no-show-notifications
        --no-show-outputs
        --no-show-separators       Do not print stack separators
    -V, --version                  Prints version information

OPTIONS:
    -s, --since <since>    When to start fetching data from. This could be a timestamp, text string, or the words
                           `today` or `yesterday`
        --sound <sound>     [default: Ping]

ARGS:
    <stack-names>...    Name of the stacks to tail
```
