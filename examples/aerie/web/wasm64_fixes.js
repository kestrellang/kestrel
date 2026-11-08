// Linked in with --post-js, so it runs inside the module's scope.
//
// Emscripten's beforeunload handler (registered by SDL) passes the C
// callback's returned pointer straight to UTF8ToString. On wasm64 that
// pointer is a BigInt, which UTF8ToString rejects, aborting on every page
// unload. Accept BigInt pointers by converting them first.
{
  const utf8ToString = UTF8ToString;
  UTF8ToString = (ptr, ...rest) => utf8ToString(typeof ptr === 'bigint' ? Number(ptr) : ptr, ...rest);
}
