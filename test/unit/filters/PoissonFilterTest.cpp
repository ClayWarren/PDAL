/******************************************************************************
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following
 * conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above
 *       copyright notice, this list of conditions and the following
 *       disclaimer in the documentation and/or other materials provided
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

#include <rust/pdal-capi/include/pdal_capi.h>

namespace pdal
{

TEST(PoissonFilterTest, partialNormalsThrow)
{
    // Only NormalX is present: the filter must reject the layout.
    EXPECT_EQ(
        pdal_filter_poisson_validate_normals(true, false, false), -1);
    EXPECT_EQ(
        pdal_filter_poisson_validate_normals(false, true, false), -1);
    EXPECT_EQ(
        pdal_filter_poisson_validate_normals(true, true, false), -1);
}

TEST(PoissonFilterTest, registersMissingNormalDimensions)
{
    // No normal dims present: validation succeeds and the filter must
    // request that all three be registered.
    EXPECT_EQ(
        pdal_filter_poisson_validate_normals(false, false, false), 0);
    EXPECT_TRUE(
        pdal_filter_poisson_needs_normal_dims(false, false, false));

    // All three present: validation succeeds and no new dims need adding.
    EXPECT_EQ(
        pdal_filter_poisson_validate_normals(true, true, true), 0);
    EXPECT_FALSE(
        pdal_filter_poisson_needs_normal_dims(true, true, true));
}

} // namespace pdal
