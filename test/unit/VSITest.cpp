/******************************************************************************
 * Copyright (c) 2025, Norman Barker (norman.barker@gmail.com)
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following
 * conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright
 *       notice, this list of conditions and the following disclaimer in
 *       the documentation and/or other materials provided
 *       with the distribution.
 *     * Neither the name of Hobu, Inc. nor the names of its contributors may
 *       be used to endorse or promote products derived from this software
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
 * LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 ****************************************************************************/

#include <pdal/pdal_test_main.hpp>

#include "Support.hpp"

#include <pdal_capi.h>
#include <vendor/nlohmann/nlohmann/json.hpp>

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

NL::json runScenario(const char* scenario, uint64_t bufferSize)
{
    Support::Tempfile temp(true);
    char* raw = pdal_vsi_local_io_scenario_json(temp.filename().c_str(),
                                                scenario, bufferSize);
    EXPECT_NE(raw, nullptr) << (pdal_last_error() ? pdal_last_error() : "");
    return NL::json::parse(takeString(raw));
}

} // namespace

TEST(VSITest, test_tells)
{
    NL::json result = runScenario("tells", 2);
    EXPECT_EQ(result["tell_after_test"], 4);
    EXPECT_EQ(result["tell_after_digits"], 9);
    EXPECT_EQ(result["file_exists"], true);
    EXPECT_EQ(result["file_size"], 9);
    EXPECT_EQ(result["tell_after_one"], 1);
    EXPECT_EQ(result["tell_after_est"], 4);
    EXPECT_EQ(result["est"], "EST");
    EXPECT_EQ(result["digits"], "12345");
    EXPECT_EQ(result["eof_tell"], -1);
}

TEST(VSITest, test_seeks_small_buffer)
{
    NL::json result = runScenario("seeks_small_buffer", 2);
    EXPECT_EQ(result["tell_after_test"], 14);
    EXPECT_EQ(result["tell_after_digits"], 6);
    EXPECT_EQ(result["file_exists"], true);
    EXPECT_EQ(result["file_size"], 14);
    EXPECT_EQ(result["tail"], "TEST");
    EXPECT_EQ(result["eof_tell"], -1);
    EXPECT_EQ(result["good_after_eof"], false);
    EXPECT_EQ(result["tell_after_digits_read"], 6);
    EXPECT_EQ(result["digits"], "12345");
}

TEST(VSITest, test_seeks_large_buffer)
{
    NL::json result = runScenario("seeks_large_buffer", 1024);
    EXPECT_EQ(result["tell_after_test"], 14);
    EXPECT_EQ(result["tell_after_digits"], 116);
    EXPECT_EQ(result["file_exists"], true);
    EXPECT_EQ(result["file_size"], 116);
    EXPECT_EQ(result["test"], "TEST");
    EXPECT_EQ(result["digits"], "12345");
    EXPECT_EQ(result["eof_tell"], -1);
}
