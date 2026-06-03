/******************************************************************************
 * Copyright (c) 2011, Michael P. Gerlek (mpg@flaxen.com)
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
 *     * Neither the name of Hobu, Inc. or Flaxen Geo Consulting nor the
 *       names of its contributors may be used to endorse or promote
 *       products derived from this software without specific prior
 *       written permission.
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

#include <pdal/pdal_test_main.hpp>

#include "Support.hpp"

#include <pdal/PipelineManager.hpp>
#include <pdal/Stage.hpp>
#include <pdal/StageFactory.hpp>
#include <pdal/util/FileUtils.hpp>
#include <pdal_capi.h>

#include <fstream>
#include <sstream>

using namespace pdal;

TEST(PipelineManagerTest, basic)
{
    std::string outfile = Support::temppath("temp.las");
    FileUtils::deleteFile(outfile);

    std::string json =
        "{\"pipeline\":[{\"type\":\"readers.las\",\"filename\":\"" +
        Support::datapath("las/1.2-with-color.las") +
        "\"},{\"type\":\"writers.las\",\"filename\":\"" + outfile + "\"}]}";
    pdal_pipeline_t* pipeline = pdal_pipeline_create_json(json.c_str());
    ASSERT_NE(pipeline, nullptr);

    int64_t np = pdal_pipeline_execute_count(pipeline, nullptr);
    EXPECT_EQ(np, 1065);
    pdal_pipeline_destroy(pipeline);

    EXPECT_TRUE(!std::ifstream(outfile).fail());
    FileUtils::deleteFile(outfile);
}

// Make sure that when we add an option at the command line, it overrides
// a pipeline option.
TEST(PipelineManagerTest, OptionOrder)
{
    std::string cmd = Support::binpath(Support::exename("pdal") + " pipeline");

    std::string file(Support::configuredpath("pipeline/sort2.json"));

    std::string output;
    int stat = Utils::run_shell_command(cmd + " " + file, output);
    EXPECT_EQ(stat, 0);

    StageFactory f;
    Stage* r = f.createStage("readers.las");

    Options o;
    o.add("filename", Support::temppath("sorted.las"));
    r->setOptions(o);

    PointTable t;
    r->prepare(t);
    PointViewSet s = r->execute(t);
    EXPECT_EQ(s.size(), 1U);
    PointViewPtr v = *(s.begin());

    double prev = std::numeric_limits<double>::lowest();
    for (PointId idx = 0; idx < v->size(); ++idx)
    {
        double d = v->getFieldAs<double>(Dimension::Id::X, idx);
        EXPECT_GE(d, prev);
        prev = d;
    }
    FileUtils::deleteFile(Support::temppath("sorted.las"));

    stat = Utils::run_shell_command(
        cmd + " " + file + " --filters.sort.dimension=Y", output);
    EXPECT_EQ(stat, 0);

    Stage* r2 = f.createStage("readers.las");
    r2->setOptions(o);

    PointTable t2;
    r2->prepare(t2);
    s = r2->execute(t2);
    EXPECT_EQ(s.size(), 1U);
    v = *(s.begin());

    prev = std::numeric_limits<double>::lowest();
    for (PointId idx = 0; idx < v->size(); ++idx)
    {
        double d = v->getFieldAs<double>(Dimension::Id::Y, idx);
        EXPECT_GE(d, prev);
        prev = d;
    }
    FileUtils::deleteFile(Support::temppath("sorted.las"));
}

TEST(PipelineManagerTest, progress)
{
    std::string cmd = Support::binpath(Support::exename("pdal") + " pipeline");
    std::string file(Support::configuredpath("pipeline/sort2.json"));
    std::string progress = Support::temppath("pipeline-progress.txt");
    std::string output;

    {
        std::ofstream out(progress);
    }

    int stat = Utils::run_shell_command(
        cmd + " " + file + " --progress " + progress, output);
    EXPECT_EQ(stat, 0);

    std::string text = FileUtils::readFileIntoString(progress);
    EXPECT_NE(text.find("READYPIPELINE:pipeline"), std::string::npos);
    EXPECT_NE(text.find("DONEPIPELINE:pipeline"), std::string::npos);

    FileUtils::deleteFile(progress);
    FileUtils::deleteFile(Support::temppath("sorted.las"));
}

// Make sure that when we add an option at the command line, it overrides
// a pipeline option.
TEST(PipelineManagerTest, InputGlobbing)
{
    std::string cmd = Support::binpath(Support::exename("pdal") + " pipeline");

    std::string file(Support::configuredpath("pipeline/glob.json"));

    std::string output;
    int stat = Utils::run_shell_command(cmd + " " + file, output);
    EXPECT_EQ(stat, 0);

    StageFactory f;
    Stage* r = f.createStage("readers.las");

    Options o;
    o.add("filename", Support::temppath("globbed.las"));
    r->setOptions(o);

    PointTable t;
    r->prepare(t);
    PointViewSet s = r->execute(t);
    EXPECT_EQ(s.size(), 1U);
    PointViewPtr v = *(s.begin());

    EXPECT_EQ(v->size(), 10653U);

    FileUtils::deleteFile(Support::temppath("globbed.las"));
}

// EPT addon writer options are objects and not strings
TEST(PipelineManagerTest, objects)
{
    std::string cmd =
        Support::binpath(Support::exename("pdal") + " pipeline --validate");
    std::string file = Support::configuredpath("pipeline/ept_addon.json");

    std::string output;
    EXPECT_NO_THROW(Utils::run_shell_command(cmd + " " + file, output));
}

TEST(PipelineManagerTest, arrayPipeline)
{
    std::string file(Support::configuredpath("pipeline/array-pipeline.json"));
    std::string outfile(Support::temppath("array-pipeline.las"));
    FileUtils::deleteFile(outfile);

    std::string json = FileUtils::readFileIntoString(file);
    pdal_pipeline_t* pipeline = pdal_pipeline_create_json(json.c_str());
    ASSERT_NE(pipeline, nullptr);

    int64_t np = pdal_pipeline_execute_count(pipeline, nullptr);
    EXPECT_EQ(np, 10653);
    pdal_pipeline_destroy(pipeline);

    EXPECT_TRUE(!std::ifstream(outfile).fail());
    FileUtils::deleteFile(outfile);
}

TEST(PipelineManagerTest, jsonPipelineAllowsComments)
{
    std::string in = R"(
        {
            // comment before stage list
            "pipeline": [
                "in.las",
                {"type": "filters.head", "count": 1},
                "out.las"
            ]
        }
    )";

    PipelineManager mgr;
    std::istringstream iss(in);
    EXPECT_NO_THROW(mgr.readPipeline(iss));
}

TEST(PipelineManagerTest, jsonPipelineRejectsInvalidStageMetadata)
{
    std::string in = R"(
        {
            "pipeline": [
                {"type": 42, "filename": "in.las"}
            ]
        }
    )";

    PipelineManager mgr;
    std::istringstream iss(in);
    EXPECT_THROW(mgr.readPipeline(iss), pdal_error);
}

TEST(PipelineManagerTest, replace)
{
    pdal_pipeline_t* pipeline = pdal_pipeline_create();
    ASSERT_NE(pipeline, nullptr);

    int64_t r = pdal_pipeline_add_stage(pipeline, pdal_stage_create_merge());
    int64_t f = pdal_pipeline_add_stage(pipeline, pdal_stage_create_merge());
    int64_t w = pdal_pipeline_add_stage(pipeline, pdal_stage_create_merge());
    ASSERT_EQ(r, 0);
    ASSERT_EQ(f, 1);
    ASSERT_EQ(w, 2);
    ASSERT_EQ(pdal_pipeline_add_dependency(pipeline, f, r), 0);
    ASSERT_EQ(pdal_pipeline_add_dependency(pipeline, w, f), 0);

    EXPECT_EQ(
        pdal_pipeline_replace_stage(pipeline, r, pdal_stage_create_merge()), 0);
    EXPECT_EQ(pdal_pipeline_input_count(pipeline, r), 0);
    EXPECT_EQ(pdal_pipeline_input_count(pipeline, f), 1);
    EXPECT_EQ(pdal_pipeline_input(pipeline, f, 0), r);
    EXPECT_EQ(pdal_pipeline_input_count(pipeline, w), 1);
    EXPECT_EQ(pdal_pipeline_input(pipeline, w, 0), f);

    EXPECT_EQ(
        pdal_pipeline_replace_stage(pipeline, f, pdal_stage_create_merge()), 0);
    EXPECT_EQ(pdal_pipeline_input_count(pipeline, r), 0);
    EXPECT_EQ(pdal_pipeline_input_count(pipeline, f), 1);
    EXPECT_EQ(pdal_pipeline_input(pipeline, f, 0), r);
    EXPECT_EQ(pdal_pipeline_input_count(pipeline, w), 1);
    EXPECT_EQ(pdal_pipeline_input(pipeline, w, 0), f);

    EXPECT_EQ(
        pdal_pipeline_replace_stage(pipeline, w, pdal_stage_create_merge()), 0);
    EXPECT_EQ(pdal_pipeline_input_count(pipeline, r), 0);
    EXPECT_EQ(pdal_pipeline_input_count(pipeline, f), 1);
    EXPECT_EQ(pdal_pipeline_input(pipeline, f, 0), r);
    EXPECT_EQ(pdal_pipeline_input_count(pipeline, w), 1);
    EXPECT_EQ(pdal_pipeline_input(pipeline, w, 0), f);

    pdal_pipeline_destroy(pipeline);
}
