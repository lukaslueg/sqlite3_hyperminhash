#include <sqlite3ext.h>
SQLITE_EXTENSION_INIT1

#include <stddef.h>

#ifdef SQLITE_DETERMINISTIC
#define WEAK_DETERMINISTIC SQLITE_DETERMINISTIC
#else
#define WEAK_DETERMINISTIC 0
#endif

void hyperminhash_step(sqlite3_context*, int, sqlite3_value**);
void hyperminhash_final(sqlite3_context*);

// The following have error-throwing impls if `serialize`-feature is inactive
void hyperminhash_zero(sqlite3_context*, int, sqlite3_value**);
void hyperminhash_serialize_final(sqlite3_context*);
void hyperminhash_deserialize(sqlite3_context*, int, sqlite3_value**);
void hyperminhash_add(sqlite3_context*, int, sqlite3_value**);
void hyperminhash_union_step(sqlite3_context*, int, sqlite3_value**);
void hyperminhash_intersection(sqlite3_context*, int, sqlite3_value**);

int init_shim(
  sqlite3 *db,
  char **pzErrMsg,
  const sqlite3_api_routines *pApi
){
  SQLITE_EXTENSION_INIT2(pApi);
  int rc;

  if (sqlite3_libversion_number() < 3008007) {
      *pzErrMsg = sqlite3_mprintf("hyperminhash requires sqlite 3.8.7 or later");
      return SQLITE_ERROR;
  }

  rc = sqlite3_create_function_v2(
          db, // db
          "hyperminhash", // zFunctionName
          -1, // nArg
          SQLITE_UTF8 | WEAK_DETERMINISTIC, // eTextRep
          NULL, // pApp
          NULL, // xFunc
          hyperminhash_step, // xStep
          hyperminhash_final, // xFinal
          NULL // xDestroy
          );
  if (rc != SQLITE_OK)
      return rc;

  rc = sqlite3_create_function_v2(
          db, // db
          "hyperminhash_zero", // zFunctionName
          0, // nArg
          SQLITE_UTF8 | WEAK_DETERMINISTIC, // eTextRep
          NULL, // pApp
          hyperminhash_zero, // xFunc
          NULL, // xStep
          NULL, // xFinal
          NULL // xDestroy
          );
  if (rc != SQLITE_OK)
      return rc;

  rc = sqlite3_create_function_v2(
          db, // db
          "hyperminhash_add", // zFunctionName
          -1, // nArg
          SQLITE_UTF8 | WEAK_DETERMINISTIC, // eTextRep
          NULL, // pApp
          hyperminhash_add, // xFunc
          NULL, // xStep
          NULL, // xFinal
          NULL // xDestroy
          );
  if (rc != SQLITE_OK)
      return rc;

  rc = sqlite3_create_function_v2(
          db, // db
          "hyperminhash_serialize", // zFunctionName
          -1, // nArg
          SQLITE_UTF8 | WEAK_DETERMINISTIC, // eTextRep
          NULL, // pApp
          NULL, // xFunc
          hyperminhash_step, // xStep
          hyperminhash_serialize_final, // xFinal
          NULL // xDestroy
          );
  if (rc != SQLITE_OK)
      return rc;

  rc = sqlite3_create_function_v2(
          db, // db
          "hyperminhash_deserialize", // zFunctionName
          1, // nArg
          SQLITE_UTF8 | WEAK_DETERMINISTIC, // eTextRep
          NULL, // pApp
          hyperminhash_deserialize, // xFunc
          NULL, // xStep
          NULL, // xFinal
          NULL // xDestroy
          );
  if (rc != SQLITE_OK)
      return rc;

  rc = sqlite3_create_function_v2(
          db, // db
          "hyperminhash_union", // zFunctionName
          1, // nArg
          SQLITE_UTF8 | WEAK_DETERMINISTIC, // eTextRep
          NULL, // pApp
          NULL, // xFunc
          hyperminhash_union_step, // xStep
          hyperminhash_serialize_final, // xFinal
          NULL // xDestroy
          );
  if (rc != SQLITE_OK)
      return rc;

  return sqlite3_create_function_v2(
          db, // db
          "hyperminhash_intersection", // zFunctionName
          2, // nArg
          SQLITE_UTF8 | WEAK_DETERMINISTIC, // eTextRep
          NULL, // pApp
          hyperminhash_intersection, // xFunc
          NULL, // xStep
          NULL, // xFinal
          NULL // xDestroy
          );
}
