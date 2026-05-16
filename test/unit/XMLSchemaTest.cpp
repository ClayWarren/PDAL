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
 *       notice in the documentation and/or other materials provided
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

#include <pdal/XMLSchema.hpp>

using namespace pdal;

namespace
{

MetadataNode findName(MetadataNode node, const std::string& name)
{
    return node.findChild([&name](MetadataNode child)
                          { return child.name() == name; });
}

} // unnamed namespace

TEST(XMLSchemaTest, roundTrip)
{
    XMLDimList dims;
    dims.push_back(XMLDim(
        DimType(Dimension::Id::X, Dimension::Type::Signed32, 0.01, 1000.0),
        "X"));
    dims.push_back(
        XMLDim(DimType(Dimension::Id::Intensity, Dimension::Type::Unsigned16),
               "Intensity"));

    MetadataNode metadata("root");
    metadata.add("source", "unit-test", "schema source");

    XMLSchema schema(dims, metadata, Orientation::DimensionMajor);
    XMLSchema parsed(schema.xml());

    EXPECT_EQ(parsed.orientation(), Orientation::DimensionMajor);

    XMLDimList parsedDims = parsed.xmlDims();
    ASSERT_EQ(parsedDims.size(), 2u);
    EXPECT_EQ(parsedDims[0].m_name, "X");
    EXPECT_EQ(parsedDims[0].m_dimType.m_type, Dimension::Type::Signed32);
    EXPECT_DOUBLE_EQ(parsedDims[0].m_dimType.m_xform.m_scale.m_val, 0.01);
    EXPECT_DOUBLE_EQ(parsedDims[0].m_dimType.m_xform.m_offset.m_val, 1000.0);
    EXPECT_EQ(parsedDims[1].m_name, "Intensity");
    EXPECT_EQ(parsedDims[1].m_dimType.m_type, Dimension::Type::Unsigned16);

    parsed.setId("X", Dimension::Id::X);
    XForm xform(0.5, -10.0);
    parsed.setXForm(Dimension::Id::X, xform);
    EXPECT_DOUBLE_EQ(parsed.xForm(Dimension::Id::X).m_scale.m_val, 0.5);
    EXPECT_DOUBLE_EQ(parsed.xForm(Dimension::Id::X).m_offset.m_val, -10.0);

    MetadataNode source = findName(parsed.getMetadata(), "source");
    ASSERT_FALSE(source.empty());
    EXPECT_EQ(source.value(), "unit-test");
}

TEST(XMLSchemaTest, legacyNames)
{
    std::string xml =
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>"
        "<pc:PointCloudSchema xmlns:pc=\"http://pointcloud.org/schemas/PC/\">"
        "<pc:dimension>"
        "<pc:position>1</pc:position>"
        "<pc:name>Chipper Point ID</pc:name>"
        "<pc:interpretation>uint32_t</pc:interpretation>"
        "</pc:dimension>"
        "<pc:dimension>"
        "<pc:position>2</pc:position>"
        "<pc:name>Unnamed field 513</pc:name>"
        "<pc:interpretation>uint32_t</pc:interpretation>"
        "</pc:dimension>"
        "<pc:orientation>point</pc:orientation>"
        "<pc:version>1.3</pc:version>"
        "</pc:PointCloudSchema>";

    XMLSchema schema(xml);
    XMLDimList dims = schema.xmlDims();

    ASSERT_EQ(dims.size(), 2u);
    EXPECT_EQ(dims[0].m_name, "Chipper:PointID");
    EXPECT_EQ(dims[1].m_name, "Chipper:BlockID");
    EXPECT_EQ(dims[0].m_position, 1u);
    EXPECT_EQ(dims[1].m_position, 2u);
}
