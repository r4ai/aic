#!/usr/bin/env -S deno run --allow-env --allow-net --allow-run --allow-read --allow-write

/**
 * @fileoverview
 * LLVM 18.1.8 installer for GitHub Actions
 *
 * - Downloads and extracts the archive below.
 * - Places the toolchain in <GITHUB_WORKSPACE>/llvm
 * - Appends <workspace>/llvm/bin to $GITHUB_PATH
 * - Appends PKG_CONFIG_PATH to $GITHUB_ENV
 */

import { $ } from "jsr:@david/dax";

const URL = Deno.env.get("LLVM_FILE_URL");
const WORKSPACE = Deno.env.get("GITHUB_WORKSPACE") ?? Deno.cwd();
const DESTDIR = `${WORKSPACE}/llvm`;

/**
 * group output the same way GitHub Actions does
 */
const group = async <T>(name: string, fn: () => Promise<T>) => {
  console.log(`::group::${name}`);
  try {
    return await fn();
  } finally {
    console.log("::endgroup::");
  }
};

//------------------------------------------------------------
// check required environment variables
//------------------------------------------------------------
if (!URL) {
  throw new Error("LLVM_FILE_URL is not set");
}

//------------------------------------------------------------
// install prerequisites
//------------------------------------------------------------
await group("Install prerequisites", async () => {
  await $`sudo apt-get -y update`.printCommand();

  // Install libtinfo5
  const deb = "libtinfo5_6.3-2ubuntu0.1_amd64.deb";
  await $`wget -q http://security.ubuntu.com/ubuntu/pool/universe/n/ncurses/${deb}`
    .printCommand();
  await $`sudo dpkg -i ${deb}`.printCommand();
});

//------------------------------------------------------------
// installation steps
//------------------------------------------------------------
await group("Prepare directories", async () => {
  await $`mkdir -p ${DESTDIR}`.printCommand();
});

await group(`Download LLVM from '${URL}'`, async () => {
  await $`curl -L --retry 3 -o llvm.tar.xz ${URL}`.printCommand();
});

await group("Extract & move", async () => {
  await $`tar -xf llvm.tar.xz --strip-components=1 -C ${DESTDIR}`
    .printCommand();
});

await group("Verify binaries", async () => {
  await $`${DESTDIR}/bin/clang --version`.printCommand();
  await $`${DESTDIR}/bin/llvm-config --version`.printCommand();
});

//------------------------------------------------------------
// expose to subsequent workflow steps
//------------------------------------------------------------
const githubPath = Deno.env.get("GITHUB_PATH");
if (githubPath) {
  await Deno.writeTextFile(githubPath, `${DESTDIR}/bin\n`, { append: true });
}

const githubEnv = Deno.env.get("GITHUB_ENV");
if (githubEnv) {
  await Deno.writeTextFile(
    githubEnv,
    [
      `PKG_CONFIG_PATH=${DESTDIR}/lib/pkgconfig`,
      `LIBRARY_PATH=${DESTDIR}/lib:$LIBRARY_PATH`,
      `LD_LIBRARY_PATH=${DESTDIR}/lib:$LD_LIBRARY_PATH`,
    ].join("\n") + "\n",
    { append: true },
  );
}

$.logStep(`LLVM ${URL} installed to ${DESTDIR}`);
