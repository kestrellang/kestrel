// GitHub Releases API client
//
// Queries the GitHub Releases API to find kestrel toolchain releases
// and download URLs for the current platform.

module jessup.github

import swoop.swoop.(Swoop)
import quill.value.(Value)
import jessup.error.(JessupError)
import jessup.platform.(Platform)

// ============================================================================
// RELEASE INFO
// ============================================================================

/// A GitHub release with its tag and matching asset URL.
public struct Release: Cloneable {
    public var tagName: String
    public var assetUrl: String

    public init(tagName tagName: String, assetUrl assetUrl: String) {
        self.tagName = tagName;
        self.assetUrl = assetUrl;
    }

    public func clone() -> Release {
        Release(tagName: self.tagName.clone(), assetUrl: self.assetUrl.clone())
    }
}

// ============================================================================
// API
// ============================================================================

func repoApi() -> String {
    "https://api.github.com/repos/kestrellang/kestrel/releases"
}

/// Builds a Swoop client with GitHub API headers and optional auth.
/// Reads GITHUB_TOKEN from the environment for private repo access.
func githubClient() -> Swoop {
    var client = Swoop();
    client = client.header("Accept", "application/vnd.github+json");
    client = client.header("User-Agent", "jessup/0.1.0");
    match getenv("GITHUB_TOKEN") {
        .Some(token) => {
            if token.byteCount > 0 {
                var auth = String();
                auth.append("Bearer ");
                auth.append(token);
                client = client.header("Authorization", auth);
            }
        },
        .None => {}
    }
    client
}

/// Fetches the latest release matching the given channel and platform.
///
/// For "stable": finds the latest non-prerelease release.
/// For "nightly": finds the latest release tagged "nightly".
/// For a specific version like "1.0.0": finds that exact tag.
public func fetchRelease(channel channel: String, platform platform: Platform) -> Result[Release, JessupError] {
    let client = githubClient();

    // A named channel (stable/preview/beta/nightly) resolves to the most recent
    // release whose tag matches that channel's convention — see
    // `tagMatchesChannel`. Anything else is treated as an explicit version tag
    // (`v<channel>`) and fetched directly, which is reliable regardless of how
    // many newer releases exist.
    if isNamedChannel(channel: channel) {
        let url = repoApi() + "?per_page=100";
        match client.fetch(url) {
            .Err(_) => return .Err(JessupError.NetworkError("failed to fetch releases")),
            .Ok(resp) => {
                if not resp.status.isSuccess() {
                    return .Err(JessupError.NetworkError("GitHub API returned status \(resp.status.code)"))
                }
                match resp.json() {
                    .Err(_) => return .Err(JessupError.ParseError("invalid JSON in releases response")),
                    .Ok(json) => return findChannelRelease(json: json, channel: channel, platform: platform)
                }
            }
        }
    } else {
        // Specific version tag
        let url = repoApi() + "/tags/v" + channel;
        match client.fetch(url) {
            .Err(_) => return .Err(JessupError.NetworkError("failed to fetch release v" + channel)),
            .Ok(resp) => {
                if not resp.status.isSuccess() {
                    return .Err(JessupError.NotFound("release v" + channel + " not found"))
                }
                match resp.json() {
                    .Err(_) => return .Err(JessupError.ParseError("invalid JSON in release response")),
                    .Ok(json) => return findAssetInRelease(json: json, platform: platform)
                }
            }
        }
    };

    return .Err(JessupError.NotFound("no matching release found for channel: " + channel))
}

/// Fetches all available release tags from GitHub.
public func fetchAllReleases() -> Result[Array[String], JessupError] {
    let client = githubClient();

    match client.fetch(repoApi()) {
        .Err(_) => .Err(JessupError.NetworkError("failed to fetch releases")),
        .Ok(resp) => {
            if not resp.status.isSuccess() {
                return .Err(JessupError.NetworkError("GitHub API returned status \(resp.status.code)"))
            };
            match resp.json() {
                .Err(_) => .Err(JessupError.ParseError("invalid JSON in releases response")),
                .Ok(json) => parseReleaseTags(json: json)
            }
        }
    }
}

/// Fetches the latest jessup binary URL for self-update.
public func fetchJessupRelease(platform platform: Platform) -> Result[String, JessupError] {
    let client = githubClient();

    let url = repoApi() + "/latest";
    match client.fetch(url) {
        .Err(_) => .Err(JessupError.NetworkError("failed to fetch latest release")),
        .Ok(resp) => {
            if not resp.status.isSuccess() {
                return .Err(JessupError.NotFound("no release found"))
            };
            match resp.json() {
                .Err(_) => .Err(JessupError.ParseError("invalid JSON in release response")),
                .Ok(json) => findJessupAsset(json: json, platform: platform)
            }
        }
    }
}

// ============================================================================
// JSON PARSING
// ============================================================================

/// Finds the matching kestrel toolchain asset in a release JSON object.
/// Looks for an asset whose name contains the platform target string.
/// Expected asset name pattern: kestrel-<target>.tar.gz
func findAssetInRelease(json json: Value, platform platform: Platform) -> Result[Release, JessupError] {
    var tagName = "";
    match json.value(forKey: "tag_name") {
        .Some(tagVal) => {
            match tagVal.asString() {
                .Some(s) => tagName = s,
                .None => return .Err(JessupError.ParseError("tag_name is not a string"))
            }
        },
        .None => return .Err(JessupError.ParseError("missing tag_name in release"))
    }

    let target = platform.assetTarget();

    match json.value(forKey: "assets") {
        .None => return .Err(JessupError.ParseError("missing assets in release")),
        .Some(assetsVal) => {
            match assetsVal.asArray() {
                .None => return .Err(JessupError.ParseError("assets is not an array")),
                .Some(assets) => {
                    var i: Int64 = 0;
                    while i < assets.count {
                        let asset = assets(unchecked: i);
                        match asset.value(forKey: "name") {
                            .Some(nameVal) => {
                                match nameVal.asString() {
                                    .Some(name) => {
                                        if stringContains(haystack: name, needle: target) and stringContains(haystack: name, needle: ".tar.gz") {
                                            // Found matching asset — get browser_download_url
                                            match asset.value(forKey: "browser_download_url") {
                                                .Some(urlVal) => {
                                                    match urlVal.asString() {
                                                        .Some(url) => {
                                                            return .Ok(Release(tagName: tagName, assetUrl: url))
                                                        },
                                                        .None => {}
                                                    }
                                                },
                                                .None => {}
                                            }
                                        }
                                    },
                                    .None => {}
                                }
                            },
                            .None => {}
                        }
                        i = i + 1
                    }
                }
            }
        }
    }

    .Err(JessupError.NotFound("no asset found for platform " + target + " in release " + tagName))
}

/// Scans a `/releases` array (newest first) and returns the most recent
/// release whose tag matches `channel` *and* carries an asset for this
/// platform. Releases that match the channel but lack a platform asset are
/// skipped, so we land on the newest actually-installable one.
func findChannelRelease(json json: Value, channel channel: String, platform platform: Platform) -> Result[Release, JessupError] {
    match json.asArray() {
        .None => return .Err(JessupError.ParseError("releases response is not an array")),
        .Some(arr) => {
            var i: Int64 = 0;
            while i < arr.count {
                let release = arr(unchecked: i);
                match release.value(forKey: "tag_name") {
                    .Some(tagVal) => {
                        match tagVal.asString() {
                            .Some(tag) => {
                                if tagMatchesChannel(tag: tag, channel: channel) {
                                    match findAssetInRelease(json: release, platform: platform) {
                                        .Ok(found) => return .Ok(found),
                                        .Err(_) => {}
                                    }
                                }
                            },
                            .None => {}
                        }
                    },
                    .None => {}
                }
                i = i + 1
            }
        }
    }

    .Err(JessupError.NotFound("no " + channel + " release found for platform " + platform.assetTarget()))
}

/// Finds the jessup binary asset in a release (for self-update).
func findJessupAsset(json json: Value, platform platform: Platform) -> Result[String, JessupError] {
    let target = platform.assetTarget();

    match json.value(forKey: "assets") {
        .None => return .Err(JessupError.ParseError("missing assets in release")),
        .Some(assetsVal) => {
            match assetsVal.asArray() {
                .None => return .Err(JessupError.ParseError("assets is not an array")),
                .Some(assets) => {
                    var i: Int64 = 0;
                    while i < assets.count {
                        let asset = assets(unchecked: i);
                        match asset.value(forKey: "name") {
                            .Some(nameVal) => {
                                match nameVal.asString() {
                                    .Some(name) => {
                                        if stringContains(haystack: name, needle: "jessup") and stringContains(haystack: name, needle: target) {
                                            match asset.value(forKey: "browser_download_url") {
                                                .Some(urlVal) => {
                                                    match urlVal.asString() {
                                                        .Some(url) => return .Ok(url),
                                                        .None => {}
                                                    }
                                                },
                                                .None => {}
                                            }
                                        }
                                    },
                                    .None => {}
                                }
                            },
                            .None => {}
                        }
                        i = i + 1
                    }
                }
            }
        }
    }

    .Err(JessupError.NotFound("no jessup binary found for platform " + target))
}

func vsixRepoApi() -> String {
    "https://api.github.com/repos/kestrellang/kestrel-vscode/releases"
}

/// Fetches the VS Code extension (.vsix) URL for the given platform from the latest release.
public func fetchVsixRelease(channel channel: String, platform platform: Platform) -> Result[String, JessupError] {
    let client = githubClient();

    // The VSIX is published on the kestrel-vscode repo.
    var url = vsixRepoApi() + "/latest";

    match client.fetch(url) {
        .Err(_) => .Err(JessupError.NetworkError("failed to fetch release for extension")),
        .Ok(resp) => {
            if not resp.status.isSuccess() {
                return .Err(JessupError.NotFound("no release found"))
            };
            match resp.json() {
                .Err(_) => .Err(JessupError.ParseError("invalid JSON in release response")),
                .Ok(json) => findVsixAsset(json: json, platform: platform)
            }
        }
    }
}

/// Finds the .vsix asset in a release for the given platform.
/// VSIX files use VS Code target names (e.g. darwin-arm64, linux-x64).
func findVsixAsset(json json: Value, platform platform: Platform) -> Result[String, JessupError] {
    let target = platform.vsceTarget();

    match json.value(forKey: "assets") {
        .None => return .Err(JessupError.ParseError("missing assets in release")),
        .Some(assetsVal) => {
            match assetsVal.asArray() {
                .None => return .Err(JessupError.ParseError("assets is not an array")),
                .Some(assets) => {
                    var i: Int64 = 0;
                    while i < assets.count {
                        let asset = assets(unchecked: i);
                        match asset.value(forKey: "name") {
                            .Some(nameVal) => {
                                match nameVal.asString() {
                                    .Some(name) => {
                                        if stringContains(haystack: name, needle: ".vsix") and stringContains(haystack: name, needle: target) {
                                            match asset.value(forKey: "browser_download_url") {
                                                .Some(urlVal) => {
                                                    match urlVal.asString() {
                                                        .Some(assetUrl) => return .Ok(assetUrl),
                                                        .None => {}
                                                    }
                                                },
                                                .None => {}
                                            }
                                        }
                                    },
                                    .None => {}
                                }
                            },
                            .None => {}
                        }
                        i = i + 1
                    }
                }
            }
        }
    }

    .Err(JessupError.NotFound("no .vsix extension found for platform " + target))
}

/// Parses an array of releases and extracts tag names.
func parseReleaseTags(json json: Value) -> Result[Array[String], JessupError] {
    match json.asArray() {
        .None => .Err(JessupError.ParseError("releases response is not an array")),
        .Some(arr) => {
            var tags = Array[String]();
            var i: Int64 = 0;
            while i < arr.count {
                let release = arr(unchecked: i);
                match release.value(forKey: "tag_name") {
                    .Some(tagVal) => {
                        match tagVal.asString() {
                            .Some(tag) => tags.append(tag),
                            .None => {}
                        }
                    },
                    .None => {}
                }
                i = i + 1
            }
            .Ok(tags)
        }
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// True for the four rolling/branch channels resolved by scanning the release
/// list. Any other string is an explicit version tag.
func isNamedChannel(channel channel: String) -> Bool {
    channel == "stable" or channel == "preview" or channel == "beta" or channel == "nightly"
}

/// Decides whether a release `tag` belongs to a named `channel`, by the tag
/// naming convention each branch publishes under:
///   - stable:  `v1.X.Y`+ from main   — major >= 1, no prerelease suffix
///   - preview: `v0.X.Y` from main    — major 0, no prerelease suffix
///   - beta:    the rolling `beta` tag (republished from the beta branch)
///   - nightly: the rolling `nightly` tag (republished from the nightly branch)
func tagMatchesChannel(tag tag: String, channel channel: String) -> Bool {
    if channel == "nightly" {
        return tag == "nightly"
    }
    if channel == "beta" {
        return tag == "beta"
    }
    if channel == "preview" {
        return tag.starts(with: "v0.") and not stringContains(haystack: tag, needle: "-")
    }
    if channel == "stable" {
        return tag.starts(with: "v")
            and not tag.starts(with: "v0.")
            and not stringContains(haystack: tag, needle: "-")
    }
    false
}

/// Checks if haystack contains needle (simple byte search).
func stringContains(haystack haystack: String, needle needle: String) -> Bool {
    let hLen = haystack.byteCount;
    let nLen = needle.byteCount;
    if nLen > hLen {
        return false
    }
    if nLen == 0 {
        return true
    }

    var i: Int64 = 0;
    while i <= hLen - nLen {
        var matched = true;
        var j: Int64 = 0;
        while j < nLen {
            if haystack.bytes(unchecked: i + j) != needle.bytes(unchecked: j) {
                matched = false;
                j = nLen
            } else {
                j = j + 1
            }
        }
        if matched {
            return true
        }
        i = i + 1
    }
    false
}
