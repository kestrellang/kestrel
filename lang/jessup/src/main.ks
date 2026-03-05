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
import clutch.arg.(Arg)
import clutch.matches.(ArgMatches)
import clutch.error.(ParseError)
import jessup.error.(JessupError)
import jessup.toolchain.(installToolchain, setDefault, listToolchains, removeToolchain, showActive, updateToolchains, selfUpdate)

// ============================================================================
// ENTRY POINT
// ============================================================================

func main() {
    let argv = getArgv();

    // Set up CLI
    var cmd = Command(name: "jessup");
    cmd.setAbout(text: "Kestrel version manager");
    cmd.setVersion(ver: "0.1.0");

    // install <version>
    var installCmd = Command(name: "install");
    installCmd.setAbout(text: "Install a toolchain (stable, nightly, or specific version)");
    var installArg = Arg(name: "channel");
    installArg.asPositional();
    installArg.help(text: "Channel or version to install (e.g., stable, nightly, 1.0.0)");
    installArg.isRequired();
    installCmd.addArg(arg: installArg);
    cmd.addSubcommand(sub: installCmd);

    // default <version>
    var defaultCmd = Command(name: "default");
    defaultCmd.setAbout(text: "Set the default toolchain");
    var defaultArg = Arg(name: "toolchain");
    defaultArg.asPositional();
    defaultArg.help(text: "Toolchain name (e.g., stable-1.0.0, nightly-2026-03-02)");
    defaultArg.isRequired();
    defaultCmd.addArg(arg: defaultArg);
    cmd.addSubcommand(sub: defaultCmd);

    // list
    var listCmd = Command(name: "list");
    listCmd.setAbout(text: "Show installed toolchains");
    cmd.addSubcommand(sub: listCmd);

    // update
    var updateCmd = Command(name: "update");
    updateCmd.setAbout(text: "Update installed channels to latest");
    cmd.addSubcommand(sub: updateCmd);

    // remove <version>
    var removeCmd = Command(name: "remove");
    removeCmd.setAbout(text: "Remove an installed toolchain");
    var removeArg = Arg(name: "toolchain");
    removeArg.asPositional();
    removeArg.help(text: "Toolchain to remove");
    removeArg.isRequired();
    removeCmd.addArg(arg: removeArg);
    cmd.addSubcommand(sub: removeCmd);

    // show
    var showCmd = Command(name: "show");
    showCmd.setAbout(text: "Show active toolchain and path");
    cmd.addSubcommand(sub: showCmd);

    // self update
    var selfCmd = Command(name: "self");
    selfCmd.setAbout(text: "Manage jessup itself");
    var selfUpdateCmd = Command(name: "update");
    selfUpdateCmd.setAbout(text: "Update jessup to the latest version");
    selfCmd.addSubcommand(sub: selfUpdateCmd);
    cmd.addSubcommand(sub: selfCmd);

    match cmd.parse(tokens: argv) {
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

func handleInstall(matches matches: ArgMatches) {
    // Get channel from submatches
    var channel = "stable";
    if matches.submatches.count > 0 {
        let sub = matches.submatches(unchecked: 0);
        match sub.getValue(name: "channel") {
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

func handleDefault(matches matches: ArgMatches) {
    var toolchainName = "";
    if matches.submatches.count > 0 {
        let sub = matches.submatches(unchecked: 0);
        match sub.getValue(name: "toolchain") {
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

func handleRemove(matches matches: ArgMatches) {
    var toolchainName = "";
    if matches.submatches.count > 0 {
        let sub = matches.submatches(unchecked: 0);
        match sub.getValue(name: "toolchain") {
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

func handleSelf(matches matches: ArgMatches) {
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
