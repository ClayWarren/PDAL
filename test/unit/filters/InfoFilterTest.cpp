/******************************************************************************
 * Copyright (c) 2018, Hobu Inc. (hobu@hobu.co)
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

#include <filters/InfoFilter.hpp>
#include <pdal/StageFactory.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

#include <vendor/nlohmann/nlohmann/json.hpp>

#include "Support.hpp"

namespace pdal
{

namespace
{

int rustTypeId(Dimension::Type type)
{
    switch (type)
    {
    case Dimension::Type::Unsigned8:
        return 0;
    case Dimension::Type::Unsigned16:
        return 1;
    case Dimension::Type::Unsigned32:
        return 2;
    case Dimension::Type::Unsigned64:
        return 3;
    case Dimension::Type::Signed8:
        return 4;
    case Dimension::Type::Signed16:
        return 5;
    case Dimension::Type::Signed32:
        return 6;
    case Dimension::Type::Signed64:
        return 7;
    case Dimension::Type::Float:
        return 8;
    case Dimension::Type::Double:
    case Dimension::Type::None:
        return 9;
    }
    return 9;
}

pdal_point_view_t* toRustView(PointView& view)
{
    pdal_point_layout_t* layout = pdal_point_layout_create();
    for (Dimension::Id dim : view.layout()->dims())
    {
        pdal_point_layout_register_dim(layout,
                                       view.layout()->dimName(dim).c_str(),
                                       rustTypeId(view.layout()->dimType(dim)));
    }

    pdal_point_view_t* rustView = pdal_point_view_create(layout);
    pdal_spatial_reference_t* srs =
        pdal_spatial_reference_create(view.spatialReference().getWKT().c_str());
    pdal_point_view_set_spatial_reference(rustView, srs);
    pdal_spatial_reference_destroy(srs);

    for (PointId idx = 0; idx < view.size(); ++idx)
    {
        pdal_point_view_add_point(rustView);
        for (Dimension::Id dim : view.layout()->dims())
        {
            pdal_point_view_set_f64(rustView, idx,
                                    view.layout()->dimName(dim).c_str(),
                                    view.getFieldAs<double>(dim, idx));
        }
    }
    return rustView;
}

} // namespace

NL::json runRustInfo(const char* pointSpec = nullptr,
                     const char* querySpec = nullptr)
{
    StageFactory factory;

    Stage* r = factory.createStage("readers.las");
    Options rOpts;
    rOpts.add("filename", Support::datapath("las/autzen_trim.las"));
    r->setOptions(rOpts);

    PointTable t;
    r->prepare(t);
    PointViewSet views = r->execute(t);
    PointViewPtr view = *views.begin();
    pdal_point_view_t* rustView = toRustView(*view);
    char* summary = pdal_info_summary_json(rustView, pointSpec, querySpec);
    EXPECT_NE(summary, nullptr);
    NL::json json = NL::json::parse(summary);
    pdal_string_free(summary);
    pdal_point_view_destroy(rustView);
    return json;
}

TEST(InfoFilterTest, point)
{
    struct rgb
    {
        int r;
        int g;
        int b;

        bool operator==(const rgb& o) const
        {
            return o.r == r && o.g == g && o.b == b;
        }
    };

    std::vector<rgb> vtest{{84, 102, 93}, {82, 98, 90},  {80, 96, 90},
                           {79, 96, 90},  {78, 94, 89},  {82, 98, 90},
                           {80, 98, 90},  {89, 106, 99}, {80, 100, 90},
                           {77, 93, 86}};
    std::vector<rgb> v;

    NL::json points = runRustInfo("0-9")["points"];
    EXPECT_EQ(points.size(), 10U);
    for (const NL::json& n : points)
    {
        int r = n["Red"].get<int>();
        int g = n["Green"].get<int>();
        int b = n["Blue"].get<int>();
        v.push_back({r, g, b});
    }
    for (size_t i = 0; i < vtest.size(); ++i)
        EXPECT_EQ(v[i], vtest[i]);
}

TEST(InfoFilterTest, query)
{
    std::vector<int> v;
    std::vector<int> vtest{107596, 108135, 107595, 108136, 107565,
                           107566, 108164, 108134, 107597, 108163};

    NL::json points = runRustInfo(nullptr, "636133,849000/10")["points"];
    EXPECT_EQ(points.size(), 10U);
    for (const NL::json& n : points)
        v.push_back(n["PointId"].get<int>());
    std::sort(v.begin(), v.end());
    std::sort(vtest.begin(), vtest.end());
    for (size_t i = 0; i < vtest.size(); ++i)
        EXPECT_EQ(v[i], vtest[i]);
}

TEST(InfoFilterTest, direct_bounds)
{
    NL::json bbox = runRustInfo()["bbox"];
    EXPECT_EQ(bbox["minx"].get<double>(), 636001.76);
    EXPECT_EQ(bbox["maxz"].get<double>(), 520.51);
}

TEST(InfoFilterTest, misc)
{
    NL::json m = runRustInfo();

    NL::json s = m["schema"];
    EXPECT_EQ(s.size(), 20U);
    if (s.size() == 20U)
    {
        auto orderbyname = [](const NL::json& m1, const NL::json& m2)
        {
            return m1["name"].get<std::string>() <
                   m2["name"].get<std::string>();
        };
        std::sort(s.begin(), s.end(), orderbyname);
        std::vector<std::string> dims{"Blue",
                                      "Classification",
                                      "EdgeOfFlightLine",
                                      "GpsTime",
                                      "Green",
                                      "Intensity",
                                      "KeyPoint",
                                      "NumberOfReturns",
                                      "Overlap",
                                      "PointSourceId",
                                      "Red",
                                      "ReturnNumber",
                                      "ScanAngleRank",
                                      "ScanDirectionFlag",
                                      "Synthetic",
                                      "UserData",
                                      "Withheld",
                                      "X",
                                      "Y",
                                      "Z"};

        size_t i = 0;
        for (const NL::json& dim : s)
            EXPECT_EQ(dim["name"].get<std::string>(), dims[i++]);
    }
    EXPECT_EQ(m["bbox"]["maxz"].get<double>(), 520.51);

    EXPECT_TRUE(m.contains("dimensions"));

    EXPECT_TRUE(m.contains("srs"));
    EXPECT_FALSE(m["srs"]["wkt"].get<std::string>().empty());
}

} // namespace pdal
