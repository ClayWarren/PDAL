/******************************************************************************
 * Copyright (c) 2020, Bradley J Chambers (brad.chambers@gmail.com)
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

#include <string>

#include <pdal/pdal_test_main.hpp>
#include <pdal/util/FileUtils.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

#include <array>
#include <fstream>
#include <vector>

#include "Support.hpp"

using namespace pdal;

namespace
{

void writeCloud(const std::string& filename,
                const std::vector<std::array<double, 3>>& points)
{
    std::ofstream out(filename);
    out << "X,Y,Z\n";
    for (const auto& point : points)
        out << point[0] << "," << point[1] << "," << point[2] << "\n";
}

} // namespace

TEST(Chamfer, kernel)
{
    std::string a = Support::datapath("autzen/autzen-thin.las");
    std::string b = Support::datapath("las/autzen_trim.las");

    double chamfer = 0.0;
    ASSERT_EQ(pdal_chamfer(a.c_str(), b.c_str(), &chamfer), 0)
        << pdal_last_error();
    EXPECT_NEAR(chamfer, 5.907628766e+10, 1.0e+2);
}

TEST(Chamfer, distance)
{
    std::string source = Support::temppath("chamfer-source.txt");
    std::string candidate = Support::temppath("chamfer-candidate.txt");
    FileUtils::deleteFile(source);
    FileUtils::deleteFile(candidate);

    writeCloud(source, {{{0.0, 0.0, 0.0}}});
    writeCloud(candidate, {{{1.0, 0.0, 0.0}, {0.0, 2.0, 0.0}}});

    double chamfer = 0.0;
    ASSERT_EQ(pdal_chamfer(source.c_str(), candidate.c_str(), &chamfer), 0)
        << pdal_last_error();
    EXPECT_DOUBLE_EQ(chamfer, 6.0);

    writeCloud(candidate, {{{1.0, 0.0, 0.0}, {0.0, 0.0, 3.0}}});
    ASSERT_EQ(pdal_chamfer(source.c_str(), candidate.c_str(), &chamfer), 0)
        << pdal_last_error();
    EXPECT_DOUBLE_EQ(chamfer, 11.0);

    writeCloud(source, {{{1.0, 1.0, 1.0}}});
    ASSERT_EQ(pdal_chamfer(source.c_str(), candidate.c_str(), &chamfer), 0)
        << pdal_last_error();
    EXPECT_DOUBLE_EQ(chamfer, 10.0);

    FileUtils::deleteFile(source);
    FileUtils::deleteFile(candidate);
}
