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

/// Fetches the latest release matching the given channel and platform.
///
/// For "stable": finds the latest non-prerelease release.
/// For "nightly": finds the latest release tagged "nightly".
/// For a specific version like "1.0.0": finds that exact tag.
public func fetchRelease(channel channel: String, platform platform: Platform) -> Result[Release, JessupError] {
    var client = Swoop();
    client = client.header("Accept", "application/vnd.github+json");
    client = client.header("User-Agent", "jessup/0.1.0");

    if channel == "nightly" {
        // Fetch nightly release by tag
        let url = repoApi() + "/tags/nightly";
        match client.fetch(url) {
            .Err(_) => return .Err(JessupError.NetworkError("failed to fetch nightly release")),
            .Ok(resp) => {
                if not resp.status.isSuccess() {
                    return .Err(JessupError.NotFound("nightly release not found"))
                }
                match resp.json() {
                    .Err(_) => return .Err(JessupError.ParseError("invalid JSON in release response")),
                    .Ok(json) => return findAssetInRelease(json: json, platform: platform)
                }
            }
        }
    } else if channel == "stable" {
        // Fetch latest stable release
        let url = repoApi() + "/latest";
        match client.fetch(url) {
            .Err(_) => return .Err(JessupError.NetworkError("failed to fetch latest release")),
            .Ok(resp) => {
                if not resp.status.isSuccess() {
                    return .Err(JessupError.NotFound("no stable release found"))
                }
                match resp.json() {
                    .Err(_) => return .Err(JessupError.ParseError("invalid JSON in release response")),
                    .Ok(json) => return findAssetInRelease(json: json, platform: platform)
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
    var client = Swoop();
    client = client.header("Accept", "application/vnd.github+json");
    client = client.header("User-Agent", "jessup/0.1.0");

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
    var client = Swoop();
    client = client.header("Accept", "application/vnd.github+json");
    client = client.header("User-Agent", "jessup/0.1.0");

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
    var client = Swoop();
    client = client.header("Accept", "application/vnd.github+json");
    client = client.header("User-Agent", "jessup/0.1.0");

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
func findVsixAsset(json json: Value, platform platform: Platform) -> Result[String, JessupError] {
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
