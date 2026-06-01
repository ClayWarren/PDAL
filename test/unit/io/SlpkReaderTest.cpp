#include <pdal/pdal_test_main.hpp>

#include "Support.hpp"

#include <pdal_capi.h>

#include <vendor/nlohmann/nlohmann/json.hpp>

#include <algorithm>
#include <string>

using namespace pdal;

namespace
{

std::string takeString(char* raw)
{
    if (!raw)
        return std::string();
    std::string out(raw);
    pdal_string_free(raw);
    return out;
}

NL::json readSmallAutzen()
{
    const std::string filename =
        Support::datapath("i3s/SMALL_AUTZEN_LAS_All.slpk");
    char* raw = pdal_slpk_summary_json(filename.c_str(), "intensity, returns");
    EXPECT_NE(raw, nullptr) << (pdal_last_error() ? pdal_last_error() : "");
    return NL::json::parse(takeString(raw));
}

void expectSmallAutzenSummary(const NL::json& summary)
{
    EXPECT_EQ(summary["point_count"], 106u);
    std::vector<std::string> dims =
        summary["dimensions"].get<std::vector<std::string>>();
    EXPECT_NE(std::find(dims.begin(), dims.end(), "Intensity"), dims.end());
    EXPECT_NE(std::find(dims.begin(), dims.end(), "NumberOfReturns"),
              dims.end());
    EXPECT_EQ(std::find(dims.begin(), dims.end(), "GpsTime"), dims.end());
}

} // namespace

TEST(SlpkReaderTest, read_local)
{
    expectSmallAutzenSummary(readSmallAutzen());
}

TEST(SlpkReaderTest, read_stream_local)
{
    expectSmallAutzenSummary(readSmallAutzen());
}
