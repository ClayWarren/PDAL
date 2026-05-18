/******************************************************************************
 * Copyright (c) 2016, Bradley J Chambers (brad.chambers@gmail.com)
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
#include <filters/GroupByFilter.hpp>
#include <io/LasReader.hpp>
#include <pdal/Reader.hpp>
#include <pdal/SpatialReference.hpp>

#include <map>
#include <vector>

using namespace pdal;

namespace
{

class GroupBySyntheticReader : public Reader
{
public:
    std::string getName() const override
    {
        return "readers.groupby_synthetic";
    }

private:
    void addDimensions(PointLayoutPtr layout) override
    {
        layout->registerDim(Dimension::Id::Classification);
        layout->registerDim(Dimension::Id::X);
    }

    PointViewSet run(PointViewPtr view) override
    {
        SpatialReference srs("EPSG:4326");
        srs.setEpoch(2024.0);
        PointViewPtr input(new PointView(view->table(), srs));

        const std::vector<uint8_t> classes = {2, 2, 7, 7, 7};
        for (PointId idx = 0; idx < classes.size(); ++idx)
        {
            input->setField(Dimension::Id::Classification, idx, classes[idx]);
            input->setField(Dimension::Id::X, idx, idx + 10);
        }

        PointViewSet views;
        views.insert(input);
        return views;
    }
};

} // unnamed namespace

TEST(GroupByTest, basic_test)
{
    Options ro;
    ro.add("filename", Support::datapath("las/1.2-with-color.las"));
    LasReader r;
    r.setOptions(ro);

    Options fo;
    fo.add("dimension", "Classification");

    GroupByFilter s;
    s.setOptions(fo);
    s.setInput(r);

    PointTable table;
    PointViewPtr view(new PointView(table));
    s.prepare(table);
    PointViewSet viewSet = s.execute(table);

    EXPECT_EQ(2u, viewSet.size());

    std::vector<PointViewPtr> views;
    for (auto it = viewSet.begin(); it != viewSet.end(); ++it)
        views.push_back(*it);

    EXPECT_EQ(789u, views[0]->size());
    EXPECT_EQ(276u, views[1]->size());
}

TEST(GroupByTest, preservesSpatialReferenceAcrossOutputs)
{
    SpatialReference srs("EPSG:4326");
    srs.setEpoch(2024.0);

    GroupBySyntheticReader reader;

    Options options;
    options.add("dimension", "Classification");

    GroupByFilter filter;
    filter.setOptions(options);
    filter.setInput(reader);

    PointTable outTable;
    filter.prepare(outTable);
    PointViewSet viewSet = filter.execute(outTable);
    ASSERT_EQ(viewSet.size(), 2u);

    std::map<uint8_t, PointViewPtr> viewsByClass;
    for (PointViewPtr view : viewSet)
    {
        ASSERT_GT(view->size(), 0u);
        EXPECT_EQ(view->spatialReference().getWKT(), srs.getWKT());
        EXPECT_DOUBLE_EQ(view->spatialReference().getEpoch(), srs.getEpoch());

        uint8_t classification =
            view->getFieldAs<uint8_t>(Dimension::Id::Classification, 0);
        viewsByClass[classification] = view;
        for (PointId idx = 0; idx < view->size(); ++idx)
            EXPECT_EQ(
                view->getFieldAs<uint8_t>(Dimension::Id::Classification, idx),
                classification);
    }

    ASSERT_EQ(viewsByClass.count(2), 1u);
    ASSERT_EQ(viewsByClass.count(7), 1u);
    EXPECT_EQ(viewsByClass[2]->size(), 2u);
    EXPECT_EQ(viewsByClass[7]->size(), 3u);
    EXPECT_DOUBLE_EQ(viewsByClass[2]->getFieldAs<double>(Dimension::Id::X, 0),
                     10.0);
    EXPECT_DOUBLE_EQ(viewsByClass[7]->getFieldAs<double>(Dimension::Id::X, 0),
                     12.0);
}
