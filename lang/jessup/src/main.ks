// Jessup - Kestrel version manager
//
// Usage:
//   jessup install <version>     Install a toolchain (stable, nightly, or version)
//   jessup default <version>     Set the default toolchain
//   jessup list                  Show installed toolchains
//   jessup update                Update installed channels to latest
//   jessup remove <version>      Remove an installed toolchain
//   jessup show                  Show active toolchain info
//   jessup self update           Update jessup itself

module jessup.main

import clutch.os.(getArgv)
import clutch.command.(Command)
import clutch.argument.(Argument)
import clutch.matches.(ArgumentMatches)
import clutch.error.(ParseError)
import jessup.error.(JessupError)
import jessup.toolchain.(installToolchain, setDefault, listToolchains, removeToolchain, showActive, updateToolchains, selfUpdate)

// ============================================================================
// ENTRY POINT
// ============================================================================

func main() {
    let argv = getArgv();

    var installCmd = Command("install");
    installCmd = installCmd.about("Install a toolchain (stable, nightly, or specific version)");
    installCmd = installCmd.argument(Argument("channel").toPositional().help("Channel or version to install (e.g., stable, nightly, 1.0.0)").required());

    var defaultCmd = Command("default");
    defaultCmd = defaultCmd.about("Set the default toolchain");
    defaultCmd = defaultCmd.argument(Argument("toolchain").toPositional().help("Toolchain name (e.g., stable-1.0.0, nightly-2026-03-02)").required());

    var removeCmd = Command("remove");
    removeCmd = removeCmd.about("Remove an installed toolchain");
    removeCmd = removeCmd.argument(Argument("toolchain").toPositional().help("Toolchain to remove").required());

    var selfUpdateCmd = Command("update");
    selfUpdateCmd = selfUpdateCmd.about("Update jessup to the latest version");

    var selfCmd = Command("self");
    selfCmd = selfCmd.about("Manage jessup itself");
    selfCmd = selfCmd.subcommand(selfUpdateCmd);

    var cmd = Command("jessup");
    cmd = cmd.about("Kestrel version manager");
    cmd = cmd.version("0.1.0");
    cmd = cmd.subcommand(installCmd);
    cmd = cmd.subcommand(defaultCmd);
    cmd = cmd.subcommand(Command("list").about("Show installed toolchains"));
    cmd = cmd.subcommand(Command("update").about("Update installed channels to latest"));
    cmd = cmd.subcommand(removeCmd);
    cmd = cmd.subcommand(Command("show").about("Show active toolchain and path"));
    cmd = cmd.subcommand(selfCmd);

    match cmd.parse(from: argv) {
        .Ok(matches) => {
            match matches.subcommand {
                .Some(sub) => {
                    if sub.equals("install") {
                        handleInstall(matches: matches)
                    } else if sub.equals("default") {
                        handleDefault(matches: matches)
                    } else if sub.equals("list") {
                        handleList()
                    } else if sub.equals("update") {
                        handleUpdate()
                    } else if sub.equals("remove") {
                        handleRemove(matches: matches)
                    } else if sub.equals("show") {
                        handleShow()
                    } else if sub.equals("self") {
                        handleSelf(matches: matches)
                    }
                },
                .None => {
                    let _ = println(cmd.helpText());
                }
            }
        },
        .Err(e) => {
            let _ = eprintln(e.description());
        }
    }
}

// ============================================================================
// COMMAND HANDLERS
// ============================================================================

func handleInstall(matches matches: ArgumentMatches) {
    // Get channel from submatches
    var channel = "stable";
    if matches.submatches.count > 0 {
        let sub = matches.submatches(unchecked: 0);
        match sub.value(of: "channel") {
            .Some(c) => channel = c,
            .None => {}
        }
    }

    match installToolchain(channel: channel) {
        .Ok(name) => {
            // Auto-set as default if no default yet
            match setDefault(toolchainName: name) {
                .Ok(_) => {},
                .Err(e) => {
                    let _ = eprintln(e.description());
                }
            }
        },
        .Err(e) => {
            var errMsg = String();
            errMsg.append("error: ");
            errMsg.append(e.description());
            let _ = eprintln(errMsg);
        }
    }
}

func handleDefault(matches matches: ArgumentMatches) {
    var toolchainName = "";
    if matches.submatches.count > 0 {
        let sub = matches.submatches(unchecked: 0);
        match sub.value(of: "toolchain") {
            .Some(t) => toolchainName = t,
            .None => {
                let _ = eprintln("error: toolchain name required");
                return
            }
        }
    }

    match setDefault(toolchainName: toolchainName) {
        .Ok(_) => {},
        .Err(e) => {
            var errMsg = String();
            errMsg.append("error: ");
            errMsg.append(e.description());
            let _ = eprintln(errMsg);
        }
    }
}

func handleList() {
    match listToolchains() {
        .Ok(_) => {},
        .Err(e) => {
            var errMsg = String();
            errMsg.append("error: ");
            errMsg.append(e.description());
            let _ = eprintln(errMsg);
        }
    }
}

func handleUpdate() {
    match updateToolchains() {
        .Ok(_) => {},
        .Err(e) => {
            var errMsg = String();
            errMsg.append("error: ");
            errMsg.append(e.description());
            let _ = eprintln(errMsg);
        }
    }
}

func handleRemove(matches matches: ArgumentMatches) {
    var toolchainName = "";
    if matches.submatches.count > 0 {
        let sub = matches.submatches(unchecked: 0);
        match sub.value(of: "toolchain") {
            .Some(t) => toolchainName = t,
            .None => {
                let _ = eprintln("error: toolchain name required");
                return
            }
        }
    }

    match removeToolchain(toolchainName: toolchainName) {
        .Ok(_) => {},
        .Err(e) => {
            var errMsg = String();
            errMsg.append("error: ");
            errMsg.append(e.description());
            let _ = eprintln(errMsg);
        }
    }
}

func handleShow() {
    match showActive() {
        .Ok(_) => {},
        .Err(e) => {
            var errMsg = String();
            errMsg.append("error: ");
            errMsg.append(e.description());
            let _ = eprintln(errMsg);
        }
    }
}

func handleSelf(matches matches: ArgumentMatches) {
    // Check for "self update" subcommand
    if matches.submatches.count > 0 {
        let sub = matches.submatches(unchecked: 0);
        match sub.subcommand {
            .Some(selfSub) => {
                if selfSub.equals("update") {
                    match selfUpdate() {
                        .Ok(_) => {},
                        .Err(e) => {
                            var errMsg = String();
                            errMsg.append("error: ");
                            errMsg.append(e.description());
                            let _ = eprintln(errMsg);
                        }
                    }
                    return
                }
            },
            .None => {}
        }
    }

    let _ = eprintln("usage: jessup self update");
}
