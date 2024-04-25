# Rust Latam: procedural macros workshop

original repository: [https://github.com/dtolnay/proc-macro-workshop](https://github.com/dtolnay/proc-macro-workshop)

## Contents

pass status:

  - [x]**Derive macro:** `derive(Builder)`
  - [ ]**Derive macro:** `derive(CustomDebug)`
  - [ ]**Function-like macro:** `seq!`
  - [ ]**Attribute macro:** `#[sorted]`
  - [ ]**Attribute macro:** `#[bitfield]`
  - [ ]**Project recommendations**

## How to view the author's answer?

In the case of `builder`, for example, you need to switch to `refs/solution/builder` after cloning the repository code.

```bash
➜ git clone https://github.com/dtolnay/proc-macro-workshop.git
Cloning into 'proc-macro-workshop'...
remote: Enumerating objects: 811, done.
remote: Counting objects: 100% (184/184), done.
remote: Compressing objects: 100% (90/90), done.
remote: Total 811 (delta 120), reused 97 (delta 94), pack-reused 627
Receiving objects: 100% (811/811), 535.39 KiB | 2.76 MiB/s, done.
Resolving deltas: 100% (419/419), done.
➜ cd proc-macro-workshop
➜ git fetch origin refs/solution/builder
remote: Enumerating objects: 8, done.
remote: Counting objects: 100% (5/5), done.
remote: Compressing objects: 100% (3/3), done.
remote: Total 8 (delta 2), reused 2 (delta 2), pack-reused 3
Unpacking objects: 100% (8/8), 3.40 KiB | 497.00 KiB/s, done.
From https://github.com/dtolnay/proc-macro-workshop
 * branch            refs/solution/builder -> FETCH_HEAD
➜ git status
On branch master
Your branch is up to date with 'origin/master'.

nothing to commit, working tree clean
➜ git rebase FETCH_HEAD
Successfully rebased and updated refs/heads/master.
➜ git status
On branch master
Your branch and 'origin/master' have diverged,
and have 10 and 11 different commits each, respectively.
  (use "git pull" to merge the remote branch into yours)

nothing to commit, working tree clean
➜ cat builder/src/lib.rs
```
