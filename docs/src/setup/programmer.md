# Programmer Setup

## Target audience

The programmer setup is targeted toward people making changes to the engine and the pipeline elements, people writing frontend code and rust code.

## Environment setup

### C/C++ and Rust

### NodeJs

### Docker

## Monorepo tooling

## Cloud access (Legion Labs employee or partner)

#TODO(doc): Liem can you document the sso setup :)

## Build environment docker image

At any given commit you'll be able to run in a virtually similar environment as the build machines run in. To do so you need to locally build the container image the CI uses.

> Legion labs employee or partner: you can also use the same exact image by exporting the MONOREPO_DOCKER_REGISTRY environment variable, to
> `550877636976.dkr.ecr.ca-central-1.amazonaws.com/legion-labs/legion`. You also need the [proper credentials](#Cloud-access) to be access the repository.

To build a container image at the current commit, run the `build/env/setup.sh` or `build/env/setup.bat` scripts on linux or windows respectively. If you are using WSL you can build on the image on windows and use it on WSL and inversely.

```bash

docker image ls

> Identify latest build-env:<sha1 hash>
> REPOSITORY   TAG                                        IMAGE ID       CREATED        SIZE
> build-env    d08a08f410b6d0bcdc9b696468ba88d64e3286ba   0d2c00ba2367   22 hours ago   9.91GB
> build-env    c4bf6f3cf4ca777573bdbde52e1153bba39ecd3b   caab483efbcb   2 days ago     9.91GB

docker run -it --rm build-env:<sha1 hash>

```

You can also use the `build/env/exec.sh` on linux and `build/env/exec.bat` on windows to run a command with your local checkout mapped. The commands uses `target/docker` folder to output cargo artifacts.

For example to build and run tests on linux:

```bash
./build/env/exec.sh cargo m test
```

And to cross compile to windows from linux (using the container image):

```powershell
.\build\env\exec.bat cargo m build --target x86_64-pc-windows-msvc
```