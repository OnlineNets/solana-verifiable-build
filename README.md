# Solana Verify CLI

A command line tool to build and verify solana programs. Users can ensure that the hash of the on-chain program matches the hash of the program of the given codebase. 

## Installation

In order for this CLI to work properly, you must have `docker` installed on your computer. Follow the steps here: https://docs.docker.com/engine/install/ to install Docker (based on your platform)

Once the installation is complete, make sure that the server has been started: (https://docs.docker.com/config/daemon/start/)

You will also need to install Cargo if you don't already have it.

Run the following command in your shell to install it (or visit https://rustup.rs/):
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Finally, to install the Solana Verify cli, run the following in your shell:

```
cargo install solana-verify
```

## Building Verifiable Programs

To verifiably build your Solana program, go to the directory with the workspace Cargo.toml file and run the following:
```
solana-verify build
```

If you're working in a repository with multiple programs, in order to build a specific program, `$PROGRAM_LIB_NAME`, run the following:
```
solana-verify build --library-name $PROGRAM_LIB_NAME
```
The string that's passed in must be the *lib* name and NOT the *package* name. These are usually the same, but the distinction is important.
![image](https://github.com/Ellipsis-Labs/solana-verifiable-build/assets/61092285/0427e88f-cc0f-465f-b2e9-747ea1b8d3af)


(NOTE: These commands can take up to 30 minutes if you're running on an M1 Macbook Pro. This has to do with the architecture emulation required to ensure build determinism. For best performance, it is recommended to run builds on a Linux machine running x86)

You can now print the executable hash of the program by running the following:
```
solana-verify get-executable-hash target/deploy/$PROGRAM_LIB_NAME.so
```


## Deploying Verifiable Programs

When the build completes, the executable file in `target/deploy/$PROGRAM_LIB_NAME.so` will contain the buffer to upload to the network.

In order to directly upload the program to chain (NOT RECOMMENDED), run the following:
```
solana program deploy -u $NETWORK_URL target/deploy/$PROGRAM_LIB_NAME.so --program-id $PROGRAM_ID --upgrade-authority $UPDGRADE_AUTHORITY
```
The same caveats apply as any normal deployment. See the Solana [docs](https://docs.solana.com/cli/deploy-a-program) for more details.

Once the upload is completed, you can verify that the program hash matches the executable hash computed in the previous step:
```
solana-verify get-program-hash -u $NETWORK_URL $PROGRAM_ID
```

The recommended approach for deploying program is to use (Squads V3)[https://docs.squads.so/squads-v3-docs/navigating-your-squad/developers/programs].

To upgrade a verifiable build, run the following to upload the program buffer:
```
solana program write-buffer -u $NETWORK_URL target/deploy/$PROGRAM_LIB_NAME.so
```

This command will output a `$BUFFER_ADDRESS`. Before voting you upgrade the program, verify that the following command produces an identical hash to executable hash (built from the previous step)
```
solana-verify get-buffer-hash -u $NETWORK_URL $BUFFER_ADDRESS
```


## Mainnet Verified Programs
### Phoenix
```
solana-verify verify-from-repo -um --program-id PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY https://github.com/Ellipsis-Labs/phoenix-v1
```
Final Output:
```
Executable Program Hash from repo: 7c76ba11f8742d040b1a874d943c2096f1b3a48db14d2a5b411fd5dad5d1bc2d
On-chain Program Hash: 7c76ba11f8742d040b1a874d943c2096f1b3a48db14d2a5b411fd5dad5d1bc2d
Program hash matches ✅
```

### Squads V3
```
solana-verify verify-from-repo https://github.com/Squads-Protocol/squads-mpl --commit-hash c95b7673d616c377a349ca424261872dfcf8b19d --program-id SMPLecH534NA9acpos4G6x7uf3LWbCAwZQE9e8ZekMu -um --library-name squads_mpl --bpf
```
(Note: we needed to specify the `library-name` because the Squads repo includes multiple programs. We use the `--bpf` flag because `squads_mpl` was previously verified with Anchor.)

Final Output:
```
Executable Program Hash from repo: 72da599d9ee14b2a03a23ccfa6f06d53eea4a00825ad2191929cbd78fb69205c
On-chain Program Hash: 72da599d9ee14b2a03a23ccfa6f06d53eea4a00825ad2191929cbd78fb69205c
Program hash matches ✅
```

## Example Walkthrough

After installing the CLI, we can test the program verification against the following immutable mainnet program: `2ZrriTQSVekoj414Ynysd48jyn4AX6ZF4TTJRqHfbJfn`

Check it out here: https://solana.fm/address/2ZrriTQSVekoj414Ynysd48jyn4AX6ZF4TTJRqHfbJfn?cluster=mainnet-qn1

### Verification with Docker

Run the following command:

```
solana-verify verify-from-image -e examples/hello_world/target/deploy/hello_world.so -i ellipsislabs/hello_world_verifiable_build:latest -p 2ZrriTQSVekoj414Ynysd48jyn4AX6ZF4TTJRqHfbJfn
```

This command loads up the image stored at [ellipsislabs/hello_world_verifiable_build:latest](https://hub.docker.com/layers/ellipsislabs/hello_world_verifiable_build/latest/images/sha256-d8b51c04c739999da618df4271d8d088fdcb3a0d8474044ebf434ebb993b5c7d?context=explore), and verifies that the hash of the executable path in the container is the same as the hash of the on-chain program supplied to the command. Because the build was already uploaded to an image, there is no need for a full rebuild of the executable which takes an extremely long time.

The Dockerfile that creates the image `ellipsislabs/hello_world_verifiable_build:latest` can be found in ./examples/hello_world under this repo.

Below is the expected output:

```
Verifying image: "ellipsislabs/hello_world_verifiable_build:latest", on network "https://api.mainnet-beta.solana.com" against program ID 2ZrriTQSVekoj414Ynysd48jyn4AX6ZF4TTJRqHfbJfn
Executable path in container: "examples/hello_world/target/deploy/hello_world.so"

Executable hash: 08d91368d349c2b56c712422f6d274a1e8f1946ff2ecd1dc3efc3ebace52a760
Program hash: 08d91368d349c2b56c712422f6d274a1e8f1946ff2ecd1dc3efc3ebace52a760
Executable matches on-chain program data ✅
```

### Manual Verification

To get the hash of an on-chain program, we can run the following with a given program ID:

```
solana-verify get-program-hash 2ZrriTQSVekoj414Ynysd48jyn4AX6ZF4TTJRqHfbJfn
```

Which will return the following hash:

```
08d91368d349c2b56c712422f6d274a1e8f1946ff2ecd1dc3efc3ebace52a760
```

By default, this command will strip any trailing zeros away from the program executable and run the sha256 algorithm against it to compute the hash.

To manually verify this program, one could run the following from the root of this repository, which builds a program from source code and computes a hash. _This command takes a long time because it is building the binary in a Docker container_

```
solana-verify build $PWD/examples/hello_world

```

Now we can check the resulting hash from the build.

```
solana-verify get-executable-hash ./examples/hello_world/target/deploy/hello_world.so

```

This will return the hash of the stripped executable, which should match the hash of the program data retrieved from the blockchain. 

```

08d91368d349c2b56c712422f6d274a1e8f1946ff2ecd1dc3efc3ebace52a760

```
