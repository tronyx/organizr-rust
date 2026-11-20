# Extensions

An extension is a program of another author that adds routes to Organizr. PHP Organizr runs PHP files of other authors with all the rights of the application. This application runs an extension in a sandbox. A sandbox is a closed space in which a program can reach only what the host gives it.

An extension is a WebAssembly component. WebAssembly is a format for programs that run in such a sandbox. An author can write an extension in Rust or in another language that compiles to WebAssembly components.

## What an extension can do

| An extension can | An extension cannot |
|---|---|
| Answer requests to its own routes | Read or write files |
| See the user name and the access level of the caller | Open network connections |
| Write lines to the log of Organizr | Read the database, the settings or the secrets of Organizr |
| Keep values of its own, when the Admin gave the permission `storage` | See the cookies or the API key of a caller |
| Read the time and get random numbers | Read the environment of the server |

Each call of an extension has limits. A call that passes a limit is stopped, and the caller gets the status 502.

| Limit | Value |
|---|---|
| Memory of one call | 64 MiB |
| Work of one call | About two thousand million steps of WebAssembly, which is a few seconds |
| Body of a request | 1 MiB |
| Body of an answer | 8 MiB |
| Stored values, keys and values together | 1 MiB |
| File of the component | 32 MiB |

One extension handles one request at a time.

## Install an extension

An extension is a folder in the folder `plugins` of the data folder. The name of the folder is the id of the extension: small letters, digits and hyphens.

```text
<data folder>/plugins/hello/plugin.wasm
<data folder>/plugins/hello/plugin.json
```

1. Copy the folder of the extension into the folder `plugins` of the data folder.
2. Log in as the Admin.
3. Open **Settings**, and select the tab **Plugins**. The section **Extensions** lists the folder.
4. Select **Approve** for the extension. A dialog shows what the extension can do.
5. Select **Approve and switch on**.

Approve an extension only when you trust its author. The sandbox keeps an extension away from your files and your network. But an extension sees who calls it, and it decides what its own routes answer.

The approval is for one file. Organizr stores the SHA-256 of the file, which is a fingerprint of its content. When the file changes, the extension stops, and the list says so. The Admin then approves the new file.

Only the Admin approves, switches off and deletes extensions. A Co-Admin sees the list.

## Use an extension

The routes of an extension are below `/api/v2/extension/<id>/`. For the example extension:

```sh
curl -H "token: <API key>" https://organizr.example.com/api/v2/extension/hello/hello
```

To show a page of an extension in the menu, make a tab with the address of that page, for example `api/v2/extension/hello/page`. The browser keeps the document of an extension apart from Organizr, as it does for the custom HTML of the homepage. A script of the extension cannot read the cookies of Organizr.

The file `plugin.json` names the group with the least access that can call the extension. A caller with less access gets the status 401.

## Write an extension

The folder `examples/extension-template` of the repository holds a small extension in Rust. The file `crates/plugins/wit/extension.wit` describes the interface: the function `handle` that an extension exports, and the functions of `host` that it can call.

1. Install the Rust target for WebAssembly components:

   ```sh
   rustup target add wasm32-wasip2
   ```

2. Copy the folder `examples/extension-template`, with the folder `crates/plugins/wit`. Change the path of the interface in `src/lib.rs` when you move the folders.
3. Build the component:

   ```sh
   cargo build --release --target wasm32-wasip2
   ```

4. Copy the file `target/wasm32-wasip2/release/extension_template.wasm` as `plugin.wasm` into the folder of your extension.
5. Write the file `plugin.json` next to it:

   ```json
   {
     "name": "Hello",
     "version": "1.0.0",
     "description": "Greets the caller.",
     "author": "Your name",
     "permissions": ["storage"],
     "least_access": 4
   }
   ```

| Field of `plugin.json` | Meaning |
|---|---|
| `name`, `version` | Shown in the list. Both are necessary |
| `description`, `author` | Shown in the list |
| `permissions` | The permissions that the extension asks for. This version knows `storage` |
| `least_access` | The level of the group with the least access that can call the extension: 0 is the Admin, 1 the Co-Admin, 999 everybody. Without it, the level is 1 |

An extension gets these headers of a request: `accept`, `accept-language`, `content-type` and `user-agent`. It can set these headers of its answer: `content-type`, `cache-control` and `content-disposition`.

## The routes of the management

| Route | For | Purpose |
|---|---|---|
| `GET /api/v2/extensions` | Admin and Co-Admin | The extensions of the folder, with their state |
| `POST /api/v2/extensions/{id}/enable` | Admin | Approve the file with the fields `sha256` and `permissions`, and switch the extension on |
| `POST /api/v2/extensions/{id}/disable` | Admin | Switch the extension off. Its values stay |
| `DELETE /api/v2/extensions/{id}` | Admin | Delete the folder, the approval and the values |

## A server without extensions

The runtime for extensions is a build feature with the name `wasm-plugins`. The container image has it. A server that was built without the feature has none of these routes, and the settings show no section **Extensions**.

## The marketplace

A marketplace is a list of extensions on a web server. With a marketplace, the Admin installs an extension from the settings page.

Organizr reads the address of the list from the environment variable `ORGANIZR_EXTENSION_MARKETPLACE`. Without the variable, there is no marketplace, and the settings show none. The list of the project is in the repository `tronyx/organizr-rust-marketplace`:

```yaml
environment:
  ORGANIZR_EXTENSION_MARKETPLACE: https://raw.githubusercontent.com/tronyx/organizr-rust-marketplace/main/extensions.json
```

Use only a list whose owner you trust. The owner of a list decides which files it names.

1. Open **Settings**, and select the tab **Plugins**. The section **Extensions** shows the heading **Marketplace**.
2. Select **Install** for an extension. Organizr downloads its file into the folder `plugins` of the data folder.
3. Approve the extension in the list above the marketplace, as an extension that you copied by hand.

Organizr installs a file only when it has the SHA-256 that the list names. So a file that was changed on its way, or on its server, is not installed. An install does not approve an extension.

When the list names a newer file than the installed one, the marketplace shows **Another file is installed**, and offers the install again. The new file then waits for a new approval.

### The format of the list

The list is a JSON file:

```json
{
  "extensions": [
    {
      "id": "hello",
      "name": "Hello",
      "version": "1.0.0",
      "description": "Greets the caller.",
      "author": "Your name",
      "permissions": ["storage"],
      "least_access": 4,
      "download": "https://example.com/hello/plugin.wasm",
      "sha256": "the SHA-256 of the file, as 64 small hex characters"
    }
  ]
}
```

The fields `id`, `name`, `version`, `download` and `sha256` are necessary. The other fields have the meaning that they have in `plugin.json`. An install writes them into the `plugin.json` of the extension.

| Route | For | Purpose |
|---|---|---|
| `GET /api/v2/extensions/marketplace` | Admin and Co-Admin | The extensions of the list, with what is installed |
| `POST /api/v2/extensions/marketplace/{id}` | Admin | Download and install one extension |

## What is not built

An extension cannot send requests to other services yet. The marketplace routes of PHP Organizr, `/plugins/marketplace` and `/plugins/manage/{plugin}`, are not built: they list PHP plugins, which this application cannot run.
