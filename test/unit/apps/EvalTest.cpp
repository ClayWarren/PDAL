/******************************************************************************
 * Copyright (c) 2026, Hobu Inc. (info@hobu.co)
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
 *     * Neither the name of Hobu, Inc. nor the
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

#include <pdal/util/FileUtils.hpp>
#include <pdal_capi.h>

#include <fstream>
#include <string>
#include <vector>

#include "Support.hpp"

namespace pdal
{

namespace
{

void writeCloud(const std::string& filename,
                const std::vector<int>& classifications)
{
    std::ofstream out(filename);
    out << "X,Y,Z,Classification\n";
    for (size_t i = 0; i < classifications.size(); ++i)
        out << i << ",0,0," << classifications[i] << "\n";
}

void expectContains(const std::string& text, const std::string& needle)
{
    EXPECT_NE(text.find(needle), std::string::npos) << text;
}

} // namespace

TEST(EvalTest, labelStatsReportsConfusionMetrics)
{
    std::string predicted = Support::temppath("eval-predicted.txt");
    std::string truth = Support::temppath("eval-truth.txt");
    FileUtils::deleteFile(predicted);
    FileUtils::deleteFile(truth);

    writeCloud(predicted, {0, 1, 0, 1});
    writeCloud(truth, {0, 0, 1, 1});

    char* raw = pdal_eval(predicted.c_str(), truth.c_str(), "0,1",
                          "Classification", "Classification");
    ASSERT_NE(raw, nullptr) << pdal_last_error();
    std::string report(raw);
    pdal_string_free(raw);

    expectContains(report, "\"support\":2");
    expectContains(report, "\"intersection_over_union\":0.3333333333333333");
    expectContains(report, "\"f1_score\":0.5");
    expectContains(report, "\"sensitivity\":0.5");
    expectContains(report, "\"specificity\":0.5");
    expectContains(report, "\"precision\":0.5");
    expectContains(report, "\"accuracy\":0.5");
    expectContains(report,
                   "\"mean_intersection_over_union\":0.3333333333333333");
    expectContains(report, "\"overall_accuracy\":0.5");
    expectContains(report, "\"confusion_matrix\":[[1,1,0],[1,1,0],[0,0,0]]");

    FileUtils::deleteFile(predicted);
    FileUtils::deleteFile(truth);
}

TEST(EvalTest, labelStatsHandlesLabelsWithoutSupport)
{
    std::string predicted = Support::temppath("eval-predicted-empty-label.txt");
    std::string truth = Support::temppath("eval-truth-empty-label.txt");
    FileUtils::deleteFile(predicted);
    FileUtils::deleteFile(truth);

    writeCloud(predicted, {0});
    writeCloud(truth, {0});

    char* raw = pdal_eval(predicted.c_str(), truth.c_str(), "0,1",
                          "Classification", "Classification");
    ASSERT_NE(raw, nullptr) << pdal_last_error();
    std::string report(raw);
    pdal_string_free(raw);

    expectContains(report, "\"label\":1");
    expectContains(report, "\"support\":0");
    expectContains(report, "\"intersection_over_union\":0.0");
    expectContains(report, "\"sensitivity\":0.0");
    expectContains(report, "\"precision\":0.0");

    FileUtils::deleteFile(predicted);
    FileUtils::deleteFile(truth);
}

} // namespace pdal
