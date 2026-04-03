#include "solve_shim.h"

#include "ortools/sat/cp_model.pb.h"
#include "ortools/sat/cp_model_solver.h"
#include "ortools/sat/sat_parameters.pb.h"

#include <cstdlib>
#include <cstring>

extern "C" {

int32_t cpsat_solve(
    const uint8_t* model_buf, size_t model_len,
    const uint8_t* params_buf, size_t params_len,
    uint8_t** response_buf, size_t* response_len
) {
    operations_research::sat::CpModelProto model;
    if (!model.ParseFromArray(model_buf, static_cast<int>(model_len))) {
        return 1;
    }

    operations_research::sat::SatParameters params;
    if (params_buf && params_len > 0) {
        if (!params.ParseFromArray(params_buf, static_cast<int>(params_len))) {
            return 2;
        }
    }

    auto response = operations_research::sat::SolveCpModel(model, &params);

    size_t resp_size = response.ByteSizeLong();
    *response_buf = static_cast<uint8_t*>(std::malloc(resp_size));
    if (!*response_buf) return 3;
    response.SerializeToArray(*response_buf, static_cast<int>(resp_size));
    *response_len = resp_size;
    return 0;
}

void cpsat_free_response(uint8_t* buf) {
    std::free(buf);
}

} // extern "C"
