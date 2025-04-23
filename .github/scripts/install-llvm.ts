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
  await $`sudo apt-get -y update`;
  await $`sudo apt-get -y install libtinfo5`;
});

//------------------------------------------------------------
// installation steps
//------------------------------------------------------------
await group("Prepare directories", async () => {
  await $`mkdir -p ${DESTDIR}`;
});

await group(`Download LLVM from '${URL}'`, async () => {
  await $`curl -L --retry 3 -o llvm.tar.xz ${URL}`;
});

await group("Extract & move", async () => {
  await $`tar -xf llvm.tar.xz --strip-components=1 -C ${DESTDIR}`;
});

await group("Verify binaries", async () => {
  await $`${DESTDIR}/bin/clang --version`;
  await $`${DESTDIR}/bin/llvm-config --version`;
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
    `PKG_CONFIG_PATH=${DESTDIR}/lib/pkgconfig\n`,
    { append: true },
  );
}

$.logStep(`LLVM ${URL} installed to ${DESTDIR}`);
