/******************************************************************************
 * Copyright (c) 2016, Hobu Inc., (info@hobu.co)
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
 *     * Neither the name of Hobu, Inc. nor the names of contributors
 *       may be used to endorse or promote products derived from this
 *       software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
 * OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
 * OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 ****************************************************************************/

#include <fstream>
#include <string>

#include <nlohmann/json.hpp>
#include <pdal/pdal_test_main.hpp>
#include <pdal/util/FileUtils.hpp>

#include "Support.hpp"

namespace pdal
{

namespace
{
std::string appName()
{
    return Support::binpath("pdal");
}

std::string appCommand()
{
#ifdef __APPLE__
    const std::string binDir = FileUtils::getDirectory(appName());
    const std::string libDir = FileUtils::getDirectory(binDir) + "/lib";
    return "DYLD_LIBRARY_PATH=\"" + libDir + "\" " + appName();
#else
    return appName();
#endif
}
} // unnamed namespace

TEST(PdalApp, load)
{
    std::string output;

    Utils::run_shell_command(appName() + " 2>&1", output);
    EXPECT_TRUE(output.find("Usage") != std::string::npos);

    Utils::run_shell_command(appName() + " sort 2>&1", output);
    EXPECT_TRUE(output.find("kernels.sort") != std::string::npos);

    Utils::run_shell_command(appName() + " foobar 2>&1", output);
    EXPECT_TRUE(output.find("not recognized") != std::string::npos);
}

TEST(PdalApp, log)
{
    std::string output;

    Utils::run_shell_command(appName() + " -v Debug 2>&1", output);
    EXPECT_TRUE(output.find("PDAL Debug)") != std::string::npos);

    output.clear();
    Utils::run_shell_command(appName() + " --verbose=3 2>&1", output);
    EXPECT_TRUE(output.find("PDAL Debug)") != std::string::npos);

    output.clear();
    Utils::run_shell_command(appName() + " 2>&1", output);
    EXPECT_TRUE(output.find("PDAL Debug)") == std::string::npos);

    output.clear();
    // With logtiming there should be a time after "Debug" and before the
    // closing paren.
    Utils::run_shell_command(appName() + " --logtiming 2>&1", output);
    EXPECT_TRUE(output.find("PDAL Debug)") == std::string::npos);
}

TEST(PdalApp, listCommands)
{
    std::string output;

    Utils::run_shell_command(appName() + " --list-commands 2>&1", output);
    EXPECT_TRUE(output.find("Usage") == std::string::npos);
    EXPECT_TRUE(output.find("translate") != std::string::npos);

    output.clear();
    Utils::run_shell_command(appName() + " --list-commands --showjson 2>&1",
                             output);
    EXPECT_TRUE(output.find("\"name\": \"translate\"") != std::string::npos);
    EXPECT_TRUE(output.find("\"full_name\": \"kernels.translate\"") !=
                std::string::npos);
}

TEST(PdalApp, option_file)
{
    std::string output;

    std::string baseCommand =
        appName() + " translate " + Support::datapath("las/simple.las") + " " +
        Support::temppath("out.las") + " -f filters.range ";
    std::string command;

    Utils::run_shell_command(baseCommand + " 2>&1", output);
    EXPECT_TRUE(output.find("Missing value") != std::string::npos);

    command = baseCommand + "--filters.range.option_file=" +
              Support::datapath("apps/nofile") + " 2>&1";
    Utils::run_shell_command(command, output);
    EXPECT_TRUE(output.find("Can't read") != std::string::npos);

    command = baseCommand + "--filters.range.option_file=" +
              Support::datapath("apps/good_cmd_opt") + " 2>&1";
    Utils::run_shell_command(command, output);
    EXPECT_TRUE(output.empty());

    command = baseCommand + "--filters.range.option_file=" +
              Support::datapath("apps/good_json_opt") + " 2>&1";
    Utils::run_shell_command(command, output);
    EXPECT_TRUE(output.empty());

    command = baseCommand + "--filters.range.option_file=" +
              Support::datapath("apps/bad_cmd_opt") + " 2>&1";
    Utils::run_shell_command(command, output);
    EXPECT_TRUE(output.find("Unexpected argument") != std::string::npos);

    command = baseCommand + "--filters.range.option_file=" +
              Support::datapath("apps/bad_json_opt") + " 2>&1";
    Utils::run_shell_command(command, output);
    EXPECT_TRUE(output.find("Unexpected argument") != std::string::npos);
}

TEST(PdalApp, pipeline_dims_limits_metadata_dimensions)
{
    const std::string pipelineFile = Support::temppath("pipeline-dims.json");
    const std::string metadataFile =
        Support::temppath("pipeline-dims-meta.json");
    FileUtils::deleteFile(pipelineFile);
    FileUtils::deleteFile(metadataFile);

    std::ofstream pipeline(pipelineFile);
    pipeline << R"({
        "pipeline": [
            {"type": "readers.faux", "mode": "ramp", "count": 3},
            {"type": "writers.null"}
        ]
    })";
    pipeline.close();

    std::string output;
    const std::string command = appCommand() + " pipeline " + pipelineFile +
                                " --nostream --metadata " + metadataFile +
                                " --dims Z,X 2>&1";
    ASSERT_EQ(0, Utils::run_shell_command(command, output)) << output;

    const std::string metadata = FileUtils::readFileIntoString(metadataFile);
    const nlohmann::json json = nlohmann::json::parse(metadata);
    ASSERT_TRUE(json.contains("dimension_summaries"));
    ASSERT_EQ(2u, json["dimension_summaries"].size());
    EXPECT_EQ("Z", json["dimension_summaries"][0]["name"].get<std::string>());
    EXPECT_EQ("X", json["dimension_summaries"][1]["name"].get<std::string>());
}

} // namespace pdal
