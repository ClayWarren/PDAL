/******************************************************************************
 * Copyright (c) 2026, Hobu Inc.
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

#include <array>
#include <limits>
#include <vector>

#include <pdal/Scaling.hpp>

namespace pdal
{

namespace
{

PointViewPtr makeView(PointTable& table,
                      const std::vector<std::array<double, 3>>& pts)
{
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});
    PointViewPtr view(new PointView(table));
    for (PointId i = 0; i < pts.size(); ++i)
    {
        view->setField(Dimension::Id::X, i, pts[i][0]);
        view->setField(Dimension::Id::Y, i, pts[i][1]);
        view->setField(Dimension::Id::Z, i, pts[i][2]);
    }
    return view;
}

} // unnamed namespace

TEST(ScalingTest, auto_offset_and_scale)
{
    PointTable t1;
    PointTable t2;
    PointViewSet views;
    views.insert(makeView(t1, {{{-10.0, 100.0, 2.0}}, {{30.0, 160.0, 8.0}}}));
    views.insert(makeView(t2, {{{20.0, 80.0, -4.0}}, {{40.0, 200.0, 14.0}}}));

    Scaling scaling;
    scaling.m_xXform.m_offset.m_auto = true;
    scaling.m_yXform.m_offset.m_auto = true;
    scaling.m_zXform.m_offset.m_auto = true;
    scaling.m_xXform.m_scale.m_auto = true;
    scaling.m_yXform.m_scale.m_auto = true;
    scaling.m_zXform.m_scale.m_auto = true;

    scaling.setAutoXForm(views);

    EXPECT_DOUBLE_EQ(scaling.m_xXform.m_offset.m_val, 15.0);
    EXPECT_DOUBLE_EQ(scaling.m_yXform.m_offset.m_val, 140.0);
    EXPECT_DOUBLE_EQ(scaling.m_zXform.m_offset.m_val, 5.0);

    const double maxInt =
        static_cast<double>((std::numeric_limits<int>::max)());
    EXPECT_DOUBLE_EQ(scaling.m_xXform.m_scale.m_val, 25.0 / maxInt);
    EXPECT_DOUBLE_EQ(scaling.m_yXform.m_scale.m_val, 60.0 / maxInt);
    EXPECT_DOUBLE_EQ(scaling.m_zXform.m_scale.m_val, 9.0 / maxInt);
}

TEST(ScalingTest, standard_transform_is_unchanged)
{
    PointTable table;
    PointViewSet views;
    views.insert(makeView(table, {{{10.0, 20.0, 30.0}}}));

    Scaling scaling;
    scaling.setAutoXForm(views);

    EXPECT_FALSE(scaling.nonstandard());
    EXPECT_DOUBLE_EQ(scaling.m_xXform.m_scale.m_val, 1.0);
    EXPECT_DOUBLE_EQ(scaling.m_yXform.m_scale.m_val, 1.0);
    EXPECT_DOUBLE_EQ(scaling.m_zXform.m_scale.m_val, 1.0);
    EXPECT_DOUBLE_EQ(scaling.m_xXform.m_offset.m_val, 0.0);
    EXPECT_DOUBLE_EQ(scaling.m_yXform.m_offset.m_val, 0.0);
    EXPECT_DOUBLE_EQ(scaling.m_zXform.m_offset.m_val, 0.0);
}

} // namespace pdal
