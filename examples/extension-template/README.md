# Extension template

This folder holds a small extension for Organizr, written in Rust. Copy it to start an extension of your own.

The page `docs/extensions.md` of the repository tells you how to build the extension, how to install it and what an extension can do.

The example answers three routes below `/api/v2/extension/<id>/`:

| Route | Answer |
|---|---|
| `GET hello` | A greeting with the user name of the caller |
| `POST count` | The number of calls of this route. It needs the permission `storage` |
| `GET echo` | The method, the path, the query, the level of the caller and the body of the request |
