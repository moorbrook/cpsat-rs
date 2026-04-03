#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Solve a CP-SAT model. Returns 0 on success, non-zero on failure.
/// Caller owns *response_buf and must free it with cpsat_free_response.
int32_t cpsat_solve(
    const uint8_t* model_buf, size_t model_len,
    const uint8_t* params_buf, size_t params_len,
    uint8_t** response_buf, size_t* response_len
);

/// Free a response buffer allocated by cpsat_solve.
void cpsat_free_response(uint8_t* buf);

#ifdef __cplusplus
}
#endif
