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

#include <pdal_capi.h>

#include "Support.hpp"

using namespace pdal;

TEST(CSFilterTest, stageCreation)
{
    pdal_stage_t* filter = pdal_stage_create_csf(2, 1, false, nullptr, 0);
    EXPECT_NE(filter, nullptr);
    pdal_stage_destroy(filter);
}

TEST(CSFilterTest, emptyView)
{
    pdal_point_layout_t* layout = pdal_point_layout_create();
    pdal_point_layout_register_dim(layout, "X", 9);
    pdal_point_layout_register_dim(layout, "Y", 9);
    pdal_point_layout_register_dim(layout, "Z", 9);
    pdal_point_view_t* view = pdal_point_view_create(layout);

    pdal_stage_t* filter = pdal_stage_create_csf(2, 1, false, nullptr, 0);
    pdal_point_view_t* outputs[1] = {nullptr};
    EXPECT_EQ(pdal_stage_run_multi(filter, view, outputs, 1), 0u);

    pdal_stage_destroy(filter);
    pdal_point_view_destroy(view);
}

TEST(CSFilterTest, equalClassesThrowWhenOnlyGroundIsFalse)
{
    pdal_stage_t* filter = pdal_stage_create_csf(2, 2, false, nullptr, 0);
    EXPECT_EQ(filter, nullptr);
}

TEST(CSFilterTest, invalidIgnoredDimensionThrows)
{
    pdal_point_layout_t* layout = pdal_point_layout_create();
    pdal_point_layout_register_dim(layout, "X", 9);
    pdal_point_layout_register_dim(layout, "Y", 9);
    pdal_point_layout_register_dim(layout, "Z", 9);
    pdal_point_layout_register_dim(layout, "Classification", 0);
    pdal_point_view_t* view = pdal_point_view_create(layout);
    pdal_point_view_add_point(view);
    pdal_point_view_set_f64(view, 0, "X", 0.0);
    pdal_point_view_set_f64(view, 0, "Y", 0.0);
    pdal_point_view_set_f64(view, 0, "Z", 0.0);
    pdal_point_view_set_f64(view, 0, "Classification", 1);

    const char* ignored[] = {"NoSuchDim"};
    pdal_stage_t* filter = pdal_stage_create_csf(2, 1, false, ignored, 1);
    pdal_point_view_t* out = pdal_stage_run(filter, view);
    EXPECT_EQ(out, nullptr);

    pdal_stage_destroy(filter);
    pdal_point_view_destroy(view);
}
