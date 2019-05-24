#include <sqlite3ext.h>
SQLITE_EXTENSION_INIT1

#include <stddef.h>

void hyperminhash_step(sqlite3_context*, int, sqlite3_value**);
void hyperminhash_final(sqlite3_context*);

int init_shim(
  sqlite3 *db,
  char **pzErrMsg,
  const sqlite3_api_routines *pApi
){
  SQLITE_EXTENSION_INIT2(pApi);
  (void)pzErrMsg;

  return sqlite3_create_function_v2(
          db, // db
          "hyperminhash", // zFunctionName
          -1, // nArg
          SQLITE_UTF8 | SQLITE_DETERMINISTIC, // eTextRep
          NULL, // pApp
          NULL, // xFunc
          hyperminhash_step, // xStep
          hyperminhash_final, // xFinal
          NULL // xDestroy
          );
}
