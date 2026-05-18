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

#include <kernels/EvalKernel.hpp>

namespace pdal
{

TEST(EvalTest, labelStatsReportsConfusionMetrics)
{
    LabelStats stats(2);

    stats.insert(0, 0);
    stats.insert(0, 1);
    stats.insert(1, 0);
    stats.insert(1, 1);

    EXPECT_EQ(stats.getSupport(0), 2u);
    EXPECT_EQ(stats.getSupport(1), 2u);
    EXPECT_EQ(stats.getTruePositives(0), 1u);
    EXPECT_EQ(stats.getFalsePositives(0), 1u);
    EXPECT_EQ(stats.getFalseNegatives(0), 1u);
    EXPECT_EQ(stats.getTrueNegatives(0), 1u);
    EXPECT_DOUBLE_EQ(stats.getIntersectionOverUnion(0), 1.0 / 3.0);
    EXPECT_DOUBLE_EQ(stats.getF1Score(0), 0.5);
    EXPECT_DOUBLE_EQ(stats.getSensitivity(0), 0.5);
    EXPECT_DOUBLE_EQ(stats.getSpecificity(0), 0.5);
    EXPECT_DOUBLE_EQ(stats.getPrecision(0), 0.5);
    EXPECT_DOUBLE_EQ(stats.getAccuracy(0), 0.5);
    EXPECT_DOUBLE_EQ(stats.getMeanIntersectionOverUnion(), 1.0 / 3.0);
    EXPECT_DOUBLE_EQ(stats.getOverallAccuracy(), 0.5);
    EXPECT_DOUBLE_EQ(stats.getF1Score(), 0.5);
    EXPECT_EQ(stats.prettyPrintConfusionMatrix(), "[[1,1,0],[1,1,0],[0,0,0]]");
}

TEST(EvalTest, labelStatsHandlesLabelsWithoutSupport)
{
    LabelStats stats(2);

    stats.insert(0, 0);

    EXPECT_EQ(stats.getSupport(1), 0u);
    EXPECT_DOUBLE_EQ(stats.getIntersectionOverUnion(1), 0.0);
    EXPECT_DOUBLE_EQ(stats.getSensitivity(1), 0.0);
    EXPECT_DOUBLE_EQ(stats.getPrecision(1), 0.0);
}

} // namespace pdal
