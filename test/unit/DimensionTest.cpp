/******************************************************************************
 * Copyright (c) 2022, Howard Butler (info@hobu.co)
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

#include <string>

#include <rust/pdal-capi/include/pdal_capi.h>

namespace pdal
{

TEST(DimensionTest, test_sanitization)
{
    std::string with_space("Pulse width");
    char* with_space_fixed = pdal_dimension_fix_name(with_space.c_str());
    ASSERT_NE(with_space_fixed, nullptr);
    EXPECT_EQ(std::string(with_space_fixed), with_space);
    pdal_string_free(with_space_fixed);

    std::string with_a_number("DimensionName42");
    char* with_a_number_fixed = pdal_dimension_fix_name(with_a_number.c_str());
    ASSERT_NE(with_a_number_fixed, nullptr);
    EXPECT_EQ(std::string(with_a_number_fixed), with_a_number);
    pdal_string_free(with_a_number_fixed);

    std::string with_punctuation("with#punctuation.");
    char* with_punctuation_fixed =
        pdal_dimension_fix_name(with_punctuation.c_str());
    ASSERT_NE(with_punctuation_fixed, nullptr);
    EXPECT_EQ(std::string(with_punctuation_fixed), "with_punctuation_");
    pdal_string_free(with_punctuation_fixed);

    std::string begin_with_a_number("42DimensionName42");
    char* begin_with_a_number_fixed =
        pdal_dimension_fix_name(begin_with_a_number.c_str());
    ASSERT_NE(begin_with_a_number_fixed, nullptr);
    EXPECT_EQ(std::string(begin_with_a_number_fixed), "_2DimensionName42");
    pdal_string_free(begin_with_a_number_fixed);
}

} // namespace pdal
