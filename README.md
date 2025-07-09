# git-anon

A CLI tool for anonymizing git repositories before public sharing. Might be useful for niche micro-influencers of sorts.

## Features

- **Squash commits** - Combine all commits into a single anonymous commit
- **Anonymous push** - Push to remotes with anonymized commit history
- **Full clean** - Complete repository anonymization
- **Configuration** - Manage anonymous identities and remote settings
- **Safety features** - Confirmation prompts and automatic backups

## Installation

```bash
cargo install --path .
```

## Usage

### Basic Commands

```bash
# squash all commits into one anonymous commit
git-anon squash --message "Initial commit"

# push to radicle with anonymization
git-anon push rad

# full repository anonymization
git-anon clean

# configure anonymous identity
git-anon config set-identity "yourhandle" "youremail"
```

### Configuration

Configuration is stored in `~/.config/git-anon/config.toml`:

```toml
[default_identity]
name = "youremail"
email = "youremail"

[remotes.radicle]
name = "rad"
identity = "default_identity"
```

### Options

- `--yes` - Skip confirmation prompts
- `--repo <path>` - Specify repository path
- `--verbose` - Verbose output

## Safety Features

- **Automatic backups** - Creates backup branches before destructive operations
- **Confirmation prompts** - Requires user confirmation for dangerous operations
- **Uncommitted changes check** - Prevents operations on dirty repositories
- **Progress indicators** - Shows progress for long operations

## Example Workflow

1. Configure your anonymous identity:
```bash
git-anon config set-identity "Anonymous" "anonymous@example.com"
```

2. Add remote configuration:
```bash
git-anon config add-remote radicle rad
```

3. Push anonymized commits:
```bash
git-anon push rad
```

4. Or fully anonymize for one-time sharing:
```bash
git-anon clean
```

