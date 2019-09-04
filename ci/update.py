#!/usr/bin/env python3
import json
import os
import subprocess

REPO = "jgraham/arewewptyet.git"
BOT_EMAIL = "arewewptyet@users.noreply.github.com"
BOT_NAME = "arewewptyet bot"


def log(data):
    print(data)


def run(cmd, **kwargs):
    expect_error = kwargs.pop("expect_error", False)
    write_log = kwargs.pop("log", False)
    if write_log:
        log("INFO: %s" % " ".join(cmd))
    try:
        output = subprocess.check_output(cmd, **kwargs)
        if output and write_log:
            log("INFO: %s" % output)
        return output
    except subprocess.CalledProcessError as e:
        if not expect_error:
            log("ERROR: Failed with exit code %s" % e.returncode)
            if write_log:
                log(e.output)
        raise e


def git(command, *args, **kwargs):
    return run(["git", command] + list(args), **kwargs)


def cargo(command, *args, **kwargs):
    return run(["cargo", command] + list(args), **kwargs)


def main():
    src_dir = os.path.abspath(".")
    build_path = os.path.join(src_dir, "build")

    git("config", "user.email", BOT_EMAIL)
    git("config", "user.name", BOT_NAME)

    try:
        cargo("run", "--release", cwd=build_path)
    except subprocess.CalledProcessError:
        # If this process fails don't worry
        pass

    try:
        git("diff", "--exit-code", "--quiet", "--", "docs/",
            expect_error=True)
    except subprocess.CalledProcessError:
        has_changes = True
    else:
        has_changes = False

    if not has_changes:
        log("INFO: Build didn't change any data files")
        return

    git("add", "-u", "--", "docs/")
    git("commit", "-m", "Update data")

    remote_url = "https://%s@github.com/%s" % (os.environ["DEPLOY_TOKEN"],
                                               REPO)

    git("fetch", remote_url, log=False)
    git("rebase", "origin/master")
    git("push", remote_url, "HEAD:master", log=False)


if __name__ == "__main__":
    main()
