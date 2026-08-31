/******************************************************************************
* Copyright (c) 2015, Hobu Inc. (info@hobu.co)
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

#include <fstream>
#include <iostream>
#include <limits>

#include <pdal/pdal_test_main.hpp>
#include <pdal/private/gdal/Raster.hpp>

#include <io/GDALReader.hpp>
#include "Support.hpp"

using namespace pdal;

TEST(GDALReaderTest, badfile)
{
    Options ro;
    ro.add("filename", Support::datapath("png/autzen-foo.png"));

    GDALReader gr;
    gr.setOptions(ro);

    PointTable t;
    EXPECT_THROW(gr.prepare(t), pdal_error);
}


TEST(GDALReaderTest, simple)
{
    Options ro;
    ro.add("filename", Support::datapath("gdal/autzen-height.tif"));
    ro.add("header", "Intensity,Userdata,Z");

    GDALReader gr;
    gr.setOptions(ro);

    PointTable t;
    gr.prepare(t);
    PointViewSet s = gr.execute(t);
    PointViewPtr v = *s.begin();
    PointLayoutPtr l = t.layout();
    Dimension::Id id1 = l->findDim("Intensity");
    Dimension::Id id2 = l->findDim("Userdata");
    Dimension::Id id3 = l->findDim("Z");
    EXPECT_EQ(v->size(), (size_t)(735 * 973));

    auto verify = [v, id1, id2, id3]
        (PointId idx, double xx, double xy, double xr, double xg, double xb)
    {
        double r, g, b, x, y;
        x = v->getFieldAs<double>(Dimension::Id::X, idx);
        y = v->getFieldAs<double>(Dimension::Id::Y, idx);
        r = v->getFieldAs<double>(id1, idx);
        g = v->getFieldAs<double>(id2, idx);
        b = v->getFieldAs<double>(id3, idx);
        EXPECT_DOUBLE_EQ(x, xx);
        EXPECT_DOUBLE_EQ(y, xy);
        EXPECT_DOUBLE_EQ(r, xr);
        EXPECT_DOUBLE_EQ(g, xg);
        EXPECT_DOUBLE_EQ(b, xb);
    };

    verify(0, .5, .5, 0, 0, 0);
    verify(120000, 195.5, 163.5, 255, 213, 0);
    verify(290000, 410.5, 394.5, 0, 255, 206);
    verify(715154, 734.5, 972.5, 0, 0, 0);
}

TEST(GDALReaderTest, blockReadValidatesArguments)
{
    gdal::Raster raster(Support::datapath("gdal/byte.tif"));
    ASSERT_EQ(raster.open(), gdal::GDALError::None);

    std::vector<double> data;
    EXPECT_EQ(raster.read(-1, 0, 0, 1, 1, data), gdal::GDALError::InvalidBand);
    EXPECT_EQ(raster.read(raster.bandCount(), 0, 0, 1, 1, data),
              gdal::GDALError::InvalidBand);

    EXPECT_EQ(raster.read(0, 0, 0, 0, 1, data), gdal::GDALError::InvalidOption);
    EXPECT_EQ(raster.read(0, 0, 0, 1, -1, data),
              gdal::GDALError::InvalidOption);

    EXPECT_EQ(raster.read(0, -1, 0, 1, 1, data), gdal::GDALError::NoData);
    EXPECT_EQ(raster.read(0, 0, -1, 1, 1, data), gdal::GDALError::NoData);
    EXPECT_EQ(raster.read(0, raster.width(), 0, 1, 1, data),
              gdal::GDALError::NoData);
    EXPECT_EQ(raster.read(0, 0, raster.height(), 1, 1, data),
              gdal::GDALError::NoData);
}

TEST(GDALReaderTest, blockReadRejectsOverflow)
{
    gdal::Raster raster(Support::datapath("gdal/byte.tif"));
    ASSERT_EQ(raster.open(), gdal::GDALError::None);

    const int maxInt = (std::numeric_limits<int>::max)();
    std::vector<double> data;
    EXPECT_EQ(raster.read(0, 0, 0, maxInt, maxInt, data),
              gdal::GDALError::InvalidOption);
    EXPECT_EQ(raster.read(0, 0, 0, maxInt, 1, data),
              gdal::GDALError::InvalidOption);
    EXPECT_EQ(raster.read(0, 0, 0, 1, maxInt, data),
              gdal::GDALError::InvalidOption);
}

TEST(GDALReaderTest, blockReadClipsWithoutSignedOverflow)
{
    gdal::Raster raster(Support::datapath("gdal/byte.tif"));
    ASSERT_EQ(raster.open(), gdal::GDALError::None);

    std::vector<double> data;
    EXPECT_EQ(
        raster.read(0, raster.width() - 1, raster.height() - 1, 2, 2, data),
        gdal::GDALError::None);
    EXPECT_EQ(data.size(), 4U);
}

TEST(GDALReaderTest, blockReadReportsGdalFailure)
{
    Support::Tempfile vrtFile;
    std::ofstream out(vrtFile.filename());
    out << R"(<VRTDataset rasterXSize="1" rasterYSize="1">
  <GeoTransform>0, 1, 0, 0, 0, -1</GeoTransform>
  <VRTRasterBand dataType="Byte" band="1">
    <SimpleSource>
      <SourceFilename relativeToVRT="0">missing-source.tif</SourceFilename>
      <SourceBand>1</SourceBand>
      <SourceProperties RasterXSize="1" RasterYSize="1" DataType="Byte"
                        BlockXSize="1" BlockYSize="1" />
      <SrcRect xOff="0" yOff="0" xSize="1" ySize="1" />
      <DstRect xOff="0" yOff="0" xSize="1" ySize="1" />
    </SimpleSource>
  </VRTRasterBand>
</VRTDataset>)";
    out.close();

    gdal::Raster raster(vrtFile.filename());
    ASSERT_EQ(raster.open(), gdal::GDALError::None);

    std::vector<double> data;
    EXPECT_EQ(raster.read(0, 0, 0, 1, 1, data),
              gdal::GDALError::CantReadBlock);
}

struct Point
{
    double m_x;
    double m_y;
    double m_z;
};

std::ostream& operator << (std::ostream& o, const Point& p)
{
    o << p.m_x << "/" << p.m_y << "/" << p.m_z;
    return o;
}

bool operator < (const Point& p1, const Point& p2)
{
    return (p1.m_x < p2.m_x ? true :
            p1.m_x > p2.m_x ? false :
            p1.m_y < p2.m_y ? true :
            p1.m_y > p2.m_y ? false :
            p1.m_z < p2.m_z ? true : false);
}

class GDALReaderTypeTest : public ::testing::Test
{
protected:
    GDALReaderTypeTest()
    {
        std::string xyzFilename = Support::datapath("gdal/data.xyz");

        std::ifstream in(xyzFilename);

        while (true)
        {
            Point p;
            in >> p.m_x >> p.m_y >> p.m_z;
            if (in.eof())
                break;
            m_xyzPoints.push_back(p);
        }
    }

    void compare(const std::string& path)
    {
        Options ro;
        ro.add("filename", path);

        GDALReader gr;
        gr.setOptions(ro);

        PointTable t;
        gr.prepare(t);
        Dimension::Id b1 = t.layout()->findDim("band_1");
        PointViewSet s = gr.execute(t);
        PointViewPtr v = *s.begin();

        EXPECT_EQ(v->size(), m_xyzPoints.size());
        for (PointId idx = 0; idx < v->size(); ++idx)
        {
            Point p;
            p.m_x = v->getFieldAs<double>(Dimension::Id::X, idx);
            p.m_y = v->getFieldAs<double>(Dimension::Id::Y, idx);
            p.m_z = v->getFieldAs<double>(b1, idx);
            m_gdalPoints.push_back(p);
        }
        std::sort(m_xyzPoints.begin(), m_xyzPoints.end());
        std::sort(m_gdalPoints.begin(), m_gdalPoints.end());
        for (size_t i = 0; i < m_gdalPoints.size(); ++i)
        {
            EXPECT_DOUBLE_EQ(m_xyzPoints[i].m_x, m_gdalPoints[i].m_x);
            EXPECT_DOUBLE_EQ(m_xyzPoints[i].m_y, m_gdalPoints[i].m_y);
            EXPECT_DOUBLE_EQ(m_xyzPoints[i].m_z, m_gdalPoints[i].m_z);
        }

        MetadataNode l = gr.getMetadata().findChild("raster");
        if (l.empty())
            FAIL() << "Couldn't find raster metadata";
    }

private:
    std::vector<Point> m_xyzPoints;
    std::vector<Point> m_gdalPoints;
};

TEST_F(GDALReaderTypeTest, byte)
{
    compare(Support::datapath("gdal/byte.tif"));
}

TEST_F(GDALReaderTypeTest, int16)
{
    compare(Support::datapath("gdal/int16.tif"));
}

TEST_F(GDALReaderTypeTest, int32)
{
    compare(Support::datapath("gdal/int32.tif"));
}

TEST_F(GDALReaderTypeTest, float32)
{
    compare(Support::datapath("gdal/float32.tif"));
}

TEST_F(GDALReaderTypeTest, float64)
{
    compare(Support::datapath("gdal/float64.tif"));
}
