# boofi
Application with rest api performs operating system actions.

## Concept
* Contains few (and hopefully more and more) parsers to run programs or read/modify/delete files
* One local and/or multiple remote endpoints supported
* `ssh` is required for remote and `su` for local
* non-posix unsupported at the moment
* basic auth and optional bearer token
* run programs asynchronously
* parser with compatibility (OS, read/write/delete), examples, in/output parameters
* consumable for logging, automation, monitoring, ..
* tries to parse files as it is, including whitespaces (but not enforced)

```
┌───────────────────────────────────┐ ┌────────────────────────────────────┐
│                                   │ │                                    │
│  http://boofi/localhost/<action>  │ │  http://boofi/remotehost1/<action> │
│                                   │ │                                    │
└───────────▲───────────────────────┘ └───────────▲────────────────────────┘
            │                                     │
            │  ┌──────────────────────────────────┘
            │  │
            │  │  ┌───────────────────────────────────────►  [...]
            │  │  │
  ┌─────────┼──┼──┼──────────────────┐
  │         │  │  │                  │
  │         │  │  │                  │              ┌─────────────────┐
  │       ┌─┼──┼──┼─┐                │              │                 │
  │       │ │  │  │ │                │              │                 │
  │       │         │  localhost     │              │    remotehost1  │
  │       │ boofi   │                │              │                 │
  │       │         │                │ ssh          │                 │
  │       │      ───┼────────────────┼──────────────┼─►  <action>>    │
  │       │  │      │                │              │                 │
  │       │  │   ───┼────────────────┼─────┐        │                 │
  │       │  │      │                │     │        │                 │
  │       └──┼──────┘                │     │        │                 │
  │          │                       │     │        └─────────────────┘
  │          ▼                       │     │
  │        su <action>               │     │
  │                                  │     │
  └──────────────────────────────────┘     └─────────►  [...]
```



### Abstraction
```
Platform  (Linux, ..)          manages compatibility and authentication
                          ▲
                          |
                          ▼
System (ssh/ssh)               provides api for run, read, write and delete operations
                          ▲
                          |
                          ▼
Parser (curl, uptime, ..)      may deserializable input and may produces serializable output
                          ▲
                          |
                          ▼
Rest Api                       calls parser with input and sends output
```

## Logging
Default log level is `info` but can be configured via enviroment variable `RUST_LOG=<level>`.
Following levels are available: `error`, `info`, `warn`, `debug` and `trace`.

### Example
`export RUST_LOG=debug`

## configuration file
Default configuration file will be created if not exist.

### listen
```yaml
listen: 127.0.0.1:3000
```

### bearer token expiration in seconds
```yaml
max_token_expiration: 86400
```

### no SSL
```yaml
ssl: none
```

### SSL
* generate self-signed with `--self-signed-alt-names <SELF_SIGNED_ALT_NAMES>`
* use custom path with `--ssl-stored-file-path <SSL_STORED_FILE_PATH>` otherwise config file is used

#### config file
```yaml
ssl: !text
private_key: |
    -----BEGIN PRIVATE KEY-----
    MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg2NM5eV2EUGFZ6suv
    ...
    JSrw+JDAvHd3jyZ8+7Wy1/A8xYC7W426tFQedCnkByKRU5z+aFhUEmVI
    -----END PRIVATE KEY-----
    certificate: |
    -----BEGIN CERTIFICATE-----
    MIIBWDCB/6ADAgECAhQrkZ5jvY51i+tbj4DrWnz1FCVzvTAKBggqhkjOPQQDAjAh
    ...
    58hQAojT4pDVI1IXn5+zSQg8XlSM0p0+5wIhANs1ZK8ghDWaLNq1BSB2iYtWtGll
    6gA7uQG96wn6qLiT
    -----END CERTIFICATE-----
```

#### separate files
```yaml
ssl: !file
  private_key_path: /etc/boofi/cert.key
  certificate_path: /etc/boofi/cert.pem
```

## REST API
### concept
* each endpoint localhost/ssh has its own path
* authentication is not shared

#### example
```
http://localhost:3000/localhost/<path/resources>
```

### authentication
* you can choose between basic and bearer

#### basic
* ever request needs authentication
* authentication goes through `su` or `ssh`

#### bearer
##### request token
* path: `/token`
* use `get` method to generate a new token
  * basic authentication is required
* use `delete` method to delete a token
  * token authentication is required

### files
#### available file module descriptions
* path: `/files`
* shows existing file modules with their documentation
  * arguments
  * examples
  * platform compatibility
  * pattern to match files e.g. regex or absolute path

#### browses files
* path: `/files/`
* shows the directory content if path is a directory otherwise the file content
* fallback file module parser is `text`

#### read/write/delete file
* path: `/files/<target filesystem path>`
  * example: `/files/etc/passwd`
* file content is parsed via file modules
* `text` file module works as fallback and returns file content (wrapped in json)
* use http method `GET` to read, `POST` to write and `DELETE` to remove a file
    * arguments depends on the file module
* enforce a file module by using `?name=<file module name>`

### apps/programs
#### documentation
* path: `/apps`
* shows all available app modules with their usage
  * arguments
  * expected output
  * examples
  * platform compatibility
#### run
* path: `/apps/<name>`
* run a program with supported arguments
* returns structured/parsed (maybe limited) output
* use http method `POST`
* asynchronous run is supported via `?async=true`
  * it returns a task id
* a list of apps are expected

#### example
```json
[{
  "name": "ls",
  "input": {
    "path": "/tmp"
  }
}, {
  "name": "sh",
  "input": {
    "command": "ls /tmp"
  }
}]
```
 
### tasks
#### task list
* path: `/task`
* tasks are apps which runs in background and no http response is required

#### specific task
* path: `/task/<id>`

## File/App development
* check out `src/apps` or `src/files` for examples
* custom errors are located in each file/app module and needs to be converted in `src/error.rs`
* test utils are placed in `src/utils.rs`

### File
* implement `FileBuilder` and `File
* use `use crate::files::prelude::*;`
* `file_metadata!` can be useful

### App
* implement `AppBuilder` and `App`
* use `use crate::apps::prelude::*;`
* a program may needs input and may produce output. both needs to be parsed
* `app_metadata!` can be useful
