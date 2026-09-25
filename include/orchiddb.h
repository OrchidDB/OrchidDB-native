#ifndef ORCHIDDB_COMPILER_H
#define ORCHIDDB_COMPILER_H
#include <stdint.h>
#ifdef _WIN32
# define ORCHIDDB_API __declspec(dllimport)
#else
# define ORCHIDDB_API
#endif
#ifdef __cplusplus
extern "C" {
#endif
/* ABI 1. Calls may be concurrent. Compilation runs on a bounded native worker pool. */
ORCHIDDB_API uint32_t orchiddb_abi_version(void);
/* Borrowed static UTF-8 NUL-terminated strings. Do not free. */
ORCHIDDB_API const char *orchiddb_version(void);
ORCHIDDB_API const char *orchiddb_core_revision(void);
/* Input: UTF-8 NUL-terminated compiler protocol v1 JSON. Never includes source rows.
 * Output: owned UTF-8 JSON {"ok":true,"result":{version,dialect,sql,fields}}
 *      or {"ok":false,"error":"..."}. Free once using orchiddb_string_free.
 * Null/invalid UTF-8/invalid requests return an error envelope. Invalid pointers
 * are a caller error. No connection setup, execution, or implicit downloads. */
ORCHIDDB_API char *orchiddb_compile_json(const char *input);
ORCHIDDB_API void orchiddb_string_free(char *response);
#ifdef __cplusplus
}
#endif
#endif
