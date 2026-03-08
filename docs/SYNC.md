# How Sync Works in RustKanban

RustKanban works fully offline — all your tasks and tags live in a local database on your computer. Sync is completely optional. If you choose to enable it, sync lets you keep multiple devices in step with each other through a central server.

This document explains everything about how sync works, from setting it up to what happens behind the scenes.

---

## Table of Contents

- [Overview](#overview)
- [Setting Up Sync](#setting-up-sync)
- [When Does Sync Happen?](#when-does-sync-happen)
- [What Gets Synced?](#what-gets-synced)
- [How Conflicts Are Resolved](#how-conflicts-are-resolved)
- [Devices](#devices)
- [Working Offline](#working-offline)
- [Deleting Tasks and Tags](#deleting-tasks-and-tags)
- [Stale Devices](#stale-devices)
- [Limits](#limits)
- [Security and Privacy](#security-and-privacy)
- [Account Management](#account-management)
- [API Tokens](#api-tokens)
- [Export and Import](#export-and-import)
- [Logging Out](#logging-out)
- [Troubleshooting](#troubleshooting)

---

## Overview

Sync connects your local RustKanban board to a server so your tasks stay up to date across computers. Here's the basic idea:

- Each computer you use RustKanban on is registered as a **device**.
- When you open the app, it **pulls** the latest changes from the server.
- When you close the app, it **pushes** your changes to the server.
- If two devices edit the same task, the **most recent edit wins**.

Your local data is always the source of truth for your device. Even if the server goes down, you keep working as normal.

## Setting Up Sync

### Step 1: Log in

Run:

```
rk login
```

This opens your browser and asks you to sign in with GitHub. Once you authorize the app, your terminal is connected. That's it — sync is now active.

Behind the scenes, this creates a **device** (named after your computer's hostname) and a **token** that your terminal uses to talk to the server.

### Step 2 (optional): Custom device name

If you want to name the device something specific:

```
rk login --device-name "work laptop"
```

### Step 3 (optional): Custom server

By default, sync uses `https://sync.rustkanban.com`. If you host your own server:

```
rk login --server https://your-server.example.com
```

### Headless / SSH environments

If you're on a machine without a browser (like an SSH session), `rk login` will detect this and print a URL for you to open on another device. After authorizing, it shows a token and device ID that you paste back into the terminal.

You can also pass these directly:

```
rk login --token <token> --device-id <device-id>
```

## When Does Sync Happen?

Sync happens automatically at two points:

| When | What happens | Direction |
|------|-------------|-----------|
| **App opens** | Pulls the latest changes from the server | Server → You |
| **App closes** | Pushes your changes to the server | You → Server |

You can also trigger a sync manually at any time by pressing **Ctrl+R** inside the app, or from the command line:

```
rk sync
```

A manual sync does both a pull and a push in one step.

### What you'll see

- On startup, you'll briefly see "Syncing..." before the board appears.
- After a manual sync, you'll see a "Synced successfully" message at the bottom of the board.
- If something goes wrong, you'll see an error message — but you can keep working normally.

## What Gets Synced?

Everything about your tasks and tags:

**Tasks:**
- Title
- Description
- Priority (Low, Medium, High, Critical)
- Column (Todo, In Progress, Done)
- Due date
- Which tags are attached
- Whether the task is deleted

**Tags:**
- Name
- Whether the tag is deleted

**Not synced:**
- Your theme/color settings (these are personal preferences)
- Sort order preference


## How Conflicts Are Resolved

When two devices edit the same task or tag, RustKanban uses a simple rule: **the most recent edit wins**.

Every time you change a task — edit its title, move it to a different column, change its priority, or anything else — the app records a timestamp of when the change happened. When syncing, the server compares timestamps and keeps whichever version is newer.

### Example

1. You edit a task title to "Fix bug" on your laptop at 2:00 PM.
2. You edit the same task title to "Fix critical bug" on your phone at 2:05 PM.
3. When both devices sync, the server keeps "Fix critical bug" because 2:05 PM is more recent.

This applies to the entire task — not individual fields. So if you change the title on one device and the priority on another, whichever device made its change most recently "wins" for all fields.

### Tag name conflicts

Tags are unique by name. If two devices create a tag with the same name but different internal IDs, the server merges them into one tag. Any tasks using either tag will be updated to use the merged version.

## Devices

Every computer or terminal session you log in from becomes a **device**. Devices are how the server tracks which changes each of your computers has seen.

### Viewing your devices

Visit your account page at `https://sync.rustkanban.com/account` (or your custom server URL) after logging in through the browser. You'll see a table of all your devices with:

- **Name** — the device name (editable)
- **Last Synced** — when the device last connected
- **Status** — Active or Stale

You can also check your current device from the terminal:

```
rk status
```

This shows your device name, server URL, and when you last synced.

### Renaming a device

You can rename devices from the account page by editing the name field and clicking Save.

### Revoking a device

If you lose a device or want to disconnect it, click Revoke on the account page. This immediately invalidates the device's token — it can no longer sync. Your local data on that device is preserved, but it won't be able to push or pull until you run `rk login` again.

## Working Offline

RustKanban works perfectly without an internet connection. All your data lives locally. If you're offline:

- The app opens normally (the pull step is skipped with an error message).
- You can create, edit, move, and delete tasks as usual.
- When you close the app, the push step is skipped with an error message.
- Next time you're online and open the app, everything syncs up.

No changes are lost. They simply wait until the next successful sync.

## Deleting Tasks and Tags

When you delete a task or tag, it isn't immediately erased. Instead, it's marked as deleted (a "soft delete"). This is important for sync because other devices need to know that something was deleted — if the record simply disappeared, other devices would think it was a new record and re-create it.

### How deleted records are cleaned up

The server runs a background cleanup job once per day. It permanently removes deleted records only after confirming that **all active devices** have synced past the deletion timestamp. This ensures no device misses the memo.

If a device hasn't synced in a very long time (see [Stale Devices](#stale-devices)), the server doesn't wait for it.

## Stale Devices

A device is marked as **stale** if it hasn't synced in 90 days. This prevents one forgotten device from blocking cleanup of deleted records forever.

When a stale device finally syncs again:

1. The server sends it a **complete copy** of all data (not just recent changes).
2. This brings the device fully up to date.
3. The device is marked as active again.

The stale device may have missed some deletions, but the full re-sync fills in all the gaps.

## Limits

To keep the service fair for everyone, there are per-account limits:

| Resource | Limit |
|----------|-------|
| Tasks | 200 |
| Tags | 15 |
| Devices | 5 |
| API tokens | 10 |

If you hit a limit, the sync will return an error and your local changes will wait until you free up space (e.g., by deleting old tasks or revoking unused devices).

## Security and Privacy

### Authentication

Sync uses GitHub OAuth for login. RustKanban never sees your GitHub password — the login happens entirely on GitHub's website.

### Tokens

After login, your device receives a bearer token — a long random string that acts as a password for sync requests. This token:

- Is stored locally at `~/.config/rustkanban/credentials.json` with restricted file permissions (only you can read it).
- Is stored on the server as a one-way hash (the server cannot reconstruct your token from the hash).
- Expires after 90 days of inactivity (the expiry refreshes each time you sync, so regular use keeps it alive).

### Data in transit

All communication with the sync server uses HTTPS (encrypted).

### Data on the server

Your tasks and tags are stored in a PostgreSQL database on the server. If you delete your account, all server-side data is permanently erased. Your local data on your devices is not affected.

## Account Management

The account page at `https://sync.rustkanban.com/account` (sign in with GitHub) lets you:

- **View your profile** — your GitHub username.
- **Manage devices** — rename, view sync status, revoke access.
- **Manage API tokens** — create and revoke standalone tokens for scripts or integrations.
- **Export data** — download all your synced tasks and tags as a JSON file.
- **Delete your account** — permanently removes all server-side data. Local data on your devices is preserved.

## API Tokens

API tokens are standalone bearer tokens for scripting, CI pipelines, or third-party integrations. Unlike device tokens (which are created during `rk login`), API tokens are created from the account page.

Key differences from device tokens:

- API tokens are **not tied to a device** — they don't appear in the device list.
- API tokens **cannot be used for sync** — sync endpoints require a device token.
- API tokens can be set to expire in 30 days, 90 days, or never.
- You can have up to 10 API tokens.

When you create an API token, it is shown **once**. Copy it immediately — you won't be able to see it again.

## Export and Import

### Export

You can export all your data as a JSON file in two ways:

- **From the terminal:** `rk export > backup.json`
- **From the account page:** Click "Download JSON" on the Export Data section.

Both produce the same format, compatible with import.

### Import

```
rk import backup.json
```

Import is **additive** — it only adds new tasks and tags. It never modifies or deletes existing data. Tasks with the same ID as existing tasks are skipped.

## Logging Out

```
rk logout
```

This does three things:

1. Pushes any pending changes to the server (a final sync).
2. Deletes your local credentials file.
3. Your local tasks and tags are preserved — nothing is deleted from your computer.

After logging out, the app works fully offline. You can log in again at any time with `rk login`.

## Troubleshooting

### "Syncing..." hangs on startup

The app is trying to reach the server. If you're offline or the server is down, it will time out after a few seconds and continue normally.

### "Auth expired" error

Your token has expired (90+ days without syncing). Run `rk login` to get a new token.

### "Account not found" error

Your server-side account was deleted. Run `rk login` to create a new one. Your local data is still intact.

### Changes not appearing on another device

Make sure both devices have synced:
1. On the source device: close the app (triggers a push) or press Ctrl+R.
2. On the target device: reopen the app (triggers a pull) or press Ctrl+R.

### "Task limit exceeded" or "Tag limit exceeded"

You've hit the per-account limit. Delete some tasks or tags and sync again.

### Conflicting edits

If you edited the same task on two devices before either synced, the most recent edit wins. There is no merge — the entire task is overwritten by the newer version. To minimize surprises, sync frequently (Ctrl+R).
