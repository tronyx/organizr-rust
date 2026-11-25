// Writes a Brotli copy (`.br`) and a gzip copy (`.gz`) next to each text file of the built
// interface in `dist`. The server sends the copy that a browser accepts, so a page load
// moves far fewer bytes. Pictures and fonts are compressed already and get no copy. The
// list of endings matches `is_compressible` in `crates/server/src/web.rs`.

import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { brotliCompressSync, constants, gzipSync } from 'node:zlib';

const ENDINGS = ['.html', '.js', '.css', '.svg', '.json', '.txt'];

/** Below this size a copy saves too little to be worth the header. */
const SMALLEST = 1024;

const folder = fileURLToPath(new URL('../dist/', import.meta.url));

/** Every file under `path`, with its full path. */
function filesIn(path) {
  return readdirSync(path).flatMap((name) => {
    const full = join(path, name);
    return statSync(full).isDirectory() ? filesIn(full) : [full];
  });
}

let before = 0;
let after = 0;
for (const file of filesIn(folder)) {
  if (!ENDINGS.some((ending) => file.endsWith(ending))) {
    continue;
  }
  const content = readFileSync(file);
  if (content.length < SMALLEST) {
    continue;
  }
  const brotli = brotliCompressSync(content, {
    params: {
      [constants.BROTLI_PARAM_QUALITY]: constants.BROTLI_MAX_QUALITY,
      [constants.BROTLI_PARAM_SIZE_HINT]: content.length,
    },
  });
  writeFileSync(`${file}.br`, brotli);
  writeFileSync(`${file}.gz`, gzipSync(content, { level: 9 }));
  before += content.length;
  after += brotli.length;
}
console.log(
  `Compressed the text files: ${Math.round(before / 1024)} KB, ${Math.round(after / 1024)} KB with Brotli.`,
);
