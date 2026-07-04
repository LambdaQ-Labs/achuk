platform ""
    requires {} { main! : List(Str) => Try({}, [Exit(I32), ..]) }
    exposes [File, Env, Stdout]
    packages {}
    provides { "roc_main": main_for_host! }
    hosted {
        "roc_file_read": File.read!,
        "roc_env_get": Env.get!,
        "roc_stdout_line": Stdout.line!,
    }
    targets: {
        inputs_dir: "targets/",
        arm64mac: { inputs: ["libhost.a", app] },
        x64mac: { inputs: ["libhost.a", app] },
        arm64musl: { inputs: ["libhost.a", app] },
        x64musl: { inputs: ["libhost.a", app] },
    }

import File
import Env
import Stdout

main_for_host! : List(Str) => I32
main_for_host! = |args|
    match main!(args) {
        Ok({}) => 0
        Err(Exit(code)) => code
        Err(_other) => 1
    }
