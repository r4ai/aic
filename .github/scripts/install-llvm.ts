#!/usr/bin/env -S deno run --allow-env --allow-net --allow-run --allow-read --allow-write

/**
 * @fileoverview
 * LLVM installer for GitHub Actions
 *
 * - Downloads and extracts the archive below.
 * - Places the toolchain in <GITHUB_WORKSPACE>/llvm
 * - Appends <workspace>/llvm/bin to $GITHUB_PATH
 * - Appends PKG_CONFIG_PATH to $GITHUB_ENV
 */

import { $ } from "jsr:@david/dax";

type Env = {
  url: string;
  skip_install: boolean;
  workspace: string;
  destdir: string;
};

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

/**
 * Load environment variables
 */
const load_env = (): Env => {
  const url = Deno.env.get("LLVM_FILE_URL");
  if (!url) {
    throw new Error("LLVM_FILE_URL is not set");
  }
  const skip_install = Deno.env.get("LLVM_SKIP_INSTALL") === "true";
  const workspace = Deno.env.get("GITHUB_WORKSPACE") ?? Deno.cwd();
  const destdir = `${workspace}/llvm`;

  return {
    url,
    skip_install,
    workspace,
    destdir,
  };
};

/**
 * Install prerequisites
 */
const install_prerequisites = async () => {
  await group("Install prerequisites", async () => {
    await $`sudo apt-get -y update`.printCommand();

    // Install libtinfo5
    const deb = "libtinfo5_6.3-2ubuntu0.1_amd64.deb";
    await $`wget -q http://security.ubuntu.com/ubuntu/pool/universe/n/ncurses/${deb}`
      .printCommand();
    await $`sudo dpkg -i ${deb}`.printCommand();
  });
};

/**
 * Install LLVM from the given URL
 */
const install_llvm = async (env: Env) => {
  await group("Prepare directories", async () => {
    await $`mkdir -p ${env.destdir}`.printCommand();
  });

  await group(`Download LLVM from '${env.url}'`, async () => {
    await $`curl -L --retry 3 -o llvm.tar.xz ${env.url}`.printCommand();
  });

  await group("Extract & move", async () => {
    await $`tar -xf llvm.tar.xz --strip-components=1 -C ${env.destdir}`
      .printCommand();
  });

  await group("Verify binaries", async () => {
    await $`${env.destdir}/bin/clang --version`.printCommand();
    await $`${env.destdir}/bin/llvm-config --version`.printCommand();
  });
};

/**
 * Setup LLVM for GitHub Actions
 */
const setup_llvm = async (env: Env) => {
  const githubPath = Deno.env.get("GITHUB_PATH");
  if (githubPath) {
    await Deno.writeTextFile(githubPath, `${env.destdir}/bin\n`, {
      append: true,
    });
  }

  const githubEnv = Deno.env.get("GITHUB_ENV");
  if (githubEnv) {
    await Deno.writeTextFile(
      githubEnv,
      `PKG_CONFIG_PATH=${env.destdir}/lib/pkgconfig\n`,
      { append: true },
    );
  }
};

const main = async () => {
  const env = load_env();

  if (!env.skip_install) {
    await install_prerequisites();
    await install_llvm(env);
  }
  await setup_llvm(env);

  $.logStep(`LLVM ${env.url} installed to ${env.destdir}`);
};
await main();
