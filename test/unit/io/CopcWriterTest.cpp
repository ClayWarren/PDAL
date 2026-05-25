/******************************************************************************
 * Copyright (c) 2021, Hobu Inc. (info@hobu.co)
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

#include <algorithm>

#include <pdal/pdal_test_main.hpp>

#include <io/CopcReader.hpp>
#include <io/CopcWriter.hpp>
#include <io/LasReader.hpp>
#include <pdal/util/FileUtils.hpp>

#include <rust/pdal-capi/include/pdal_capi.h>

#include "Support.hpp"

#include <gdal_version.h>

namespace pdal
{

namespace
{
std::string wkt2DerivedProjected =
    "DERIVEDPROJCRS[\"Custom Site Calibrated CRS\",\n"
    "    BASEPROJCRS[\"NAD83(2011) / Mississippi East (ftUS)\",\n"
    "        BASEGEOGCRS[\"NAD83(2011)\",\n"
    "            DATUM[\"NAD83 (National Spatial Reference System "
    "2011)\",\n"
    "                ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n"
    "                    LENGTHUNIT[\"metre\",1]]],\n"
    "            PRIMEM[\"Greenwich\",0,\n"
    "                ANGLEUNIT[\"degree\",0.0174532925199433]]],\n"
    "        CONVERSION[\"SPCS83 Mississippi East zone (US Survey "
    "feet)\",\n"
    "            METHOD[\"Transverse Mercator\",\n"
    "                ID[\"EPSG\",9807]],\n"
    "            PARAMETER[\"Latitude of natural origin\",29.5,\n"
    "                ANGLEUNIT[\"degree\",0.0174532925199433],\n"
    "                ID[\"EPSG\",8801]],\n"
    "            PARAMETER[\"Longitude of natural "
    "origin\",-88.8333333333333,\n"
    "                ANGLEUNIT[\"degree\",0.0174532925199433],\n"
    "                ID[\"EPSG\",8802]],\n"
    "            PARAMETER[\"Scale factor at natural origin\",0.99995,\n"
    "                SCALEUNIT[\"unity\",1],\n"
    "                ID[\"EPSG\",8805]],\n"
    "            PARAMETER[\"False easting\",984250,\n"
    "                LENGTHUNIT[\"US survey foot\",0.304800609601219],\n"
    "                ID[\"EPSG\",8806]],\n"
    "            PARAMETER[\"False northing\",0,\n"
    "                LENGTHUNIT[\"US survey foot\",0.304800609601219],\n"
    "                ID[\"EPSG\",8807]]]],\n"
    "    DERIVINGCONVERSION[\"Affine transformation as PROJ-based\",\n"
    "        METHOD[\"PROJ-based operation method: "
    "+proj=pipeline +step +proj=unitconvert +xy_in=m +xy_out=us-ft "
    "+step +proj=affine +xoff=20 "
    "+step +proj=unitconvert +xy_in=us-ft +xy_out=m\"]],\n"
    "    CS[Cartesian,2],\n"
    "        AXIS[\"northing (Y)\",north,\n"
    "            LENGTHUNIT[\"US survey foot\",0.304800609601219]],\n"
    "        AXIS[\"easting (X)\",east,\n"
    "            LENGTHUNIT[\"US survey foot\",0.304800609601219]],\n"
    "    REMARK[\"EPSG:6507 with 20 feet offset and axis inversion\"]]";
}

TEST(CopcWriterTest, srsWkt2)
{
#if GDAL_VERSION_NUM <= GDAL_COMPUTE_VERSION(3, 6, 0)
    // not working with PROJ >= 9.2.0 https://github.com/OSGeo/gdal/pull/6800
    std::cerr << "Test disabled with GDAL <= 3.6.0" << std::endl;
    return;
#endif
    const auto filename = Support::temppath("srsWkt2.copc.las");
    {
        pdal_point_layout_t* layout = pdal_point_layout_create();
        pdal_point_layout_register_dim(layout, "X", 9);
        pdal_point_layout_register_dim(layout, "Y", 9);
        pdal_point_layout_register_dim(layout, "Z", 9);
        pdal_point_view_t* view = pdal_point_view_create(layout);
        pdal_point_view_add_point(view);
        pdal_point_view_set_f64(view, 0, "X", 635619.85);
        pdal_point_view_set_f64(view, 0, "Y", 850064.04);
        pdal_point_view_set_f64(view, 0, "Z", 447.01);

        pdal_options_t* writeOpts = pdal_options_create();
        pdal_options_add_str(writeOpts, "filename", filename.c_str());
        pdal_options_add_str(writeOpts, "a_srs", wkt2DerivedProjected.c_str());
        pdal_options_add_str(writeOpts, "enhanced_srs_vlrs", "true");
        pdal_writer_t* writer = pdal_writer_create_copc(writeOpts);
        EXPECT_NE(writer, nullptr) << pdal_last_error();
        EXPECT_TRUE(pdal_writer_write_view(writer, view)) << pdal_last_error();
        pdal_writer_destroy(writer);
        pdal_options_destroy(writeOpts);
        pdal_point_view_destroy(view);
    }

    {
        Options options;
        options.add("filename", filename);

        LasReader reader;
        reader.setOptions(options);

        const QuickInfo qi(reader.preview());
        std::string srs = qi.m_srs.getWKT();

        EXPECT_TRUE(Utils::startsWith(
            srs, "DERIVEDPROJCRS[\"Custom Site Calibrated CRS\""));
    }

    {
        Options options;
        options.add("filename", filename);
        options.add("srs_vlr_order", "projjson, wkt2");

        LasReader reader;
        reader.setOptions(options);

        const QuickInfo qi(reader.preview());
        std::string srs = qi.m_srs.getPROJJSON();

        EXPECT_TRUE(
            Utils::startsWith(srs, "{\n  \"type\": \"DerivedProjectedCRS\","));
    }
}

TEST(CopcWriterTest, srsUTM)
{
    const auto filename = Support::temppath("srs.copc.las");
    {
        pdal_point_layout_t* layout = pdal_point_layout_create();
        pdal_point_layout_register_dim(layout, "X", 9);
        pdal_point_layout_register_dim(layout, "Y", 9);
        pdal_point_layout_register_dim(layout, "Z", 9);
        pdal_point_view_t* view = pdal_point_view_create(layout);
        pdal_point_view_add_point(view);
        pdal_point_view_set_f64(view, 0, "X", 635619.85);
        pdal_point_view_set_f64(view, 0, "Y", 850064.04);
        pdal_point_view_set_f64(view, 0, "Z", 447.01);

        pdal_options_t* writeOpts = pdal_options_create();
        pdal_options_add_str(writeOpts, "filename", filename.c_str());
        pdal_options_add_str(writeOpts, "a_srs", "EPSG:26915");
        pdal_options_add_str(writeOpts, "enhanced_srs_vlrs", "true");
        pdal_writer_t* writer = pdal_writer_create_copc(writeOpts);
        EXPECT_NE(writer, nullptr) << pdal_last_error();
        EXPECT_TRUE(pdal_writer_write_view(writer, view)) << pdal_last_error();
        pdal_writer_destroy(writer);
        pdal_options_destroy(writeOpts);
        pdal_point_view_destroy(view);
    }

    Options ops;
    ops.add("filename", filename);

    LasReader r;
    r.setOptions(ops);

    PointTable t;
    r.prepare(t);
    r.execute(t);

    const QuickInfo qi(r.preview());
    std::string srs = qi.m_srs.getWKT();
    EXPECT_TRUE(
        Utils::startsWith(srs, "PROJCRS[\"NAD83 / UTM zone 15N\",BASEGEOGCRS"));

    const char* data = nullptr;

    EXPECT_TRUE(r.vlrData("LASF_Projection", 4224, data) > 0);
    EXPECT_TRUE(Utils::startsWith(data, "PROJCRS[\"NAD83 / UTM zone 15N\""));

    data = nullptr;
    EXPECT_TRUE(r.vlrData("PDAL", 4225, data) > 0);
    EXPECT_TRUE(Utils::startsWith(data, "{\n  \"type\": \"ProjectedCRS\","));

    data = nullptr;
    // This vlr data must not be null terminated and segfaults when startsWith
    // tries to read it
    EXPECT_TRUE(r.vlrData("LASF_Projection", 2112, data) > 0);
    std::string info(data, 50);
    bool test = Utils::startsWith(info, "PROJCS[\"NAD83 / UTM zone 15N\"");
    EXPECT_TRUE(test);
}

TEST(CopcWriterTest, scaling)
{
    using namespace Dimension;

    const std::string filename(Support::temppath("copc_scaling.las"));
    FileUtils::deleteFile(filename);

    // Route the write path through the Rust COPC writer C ABI so this
    // test exercises Rust-backed scale/offset and LAS 1.4 LAZ output.
    pdal_point_layout_t* layout = pdal_point_layout_create();
    pdal_point_layout_register_dim(layout, "X", 9);
    pdal_point_layout_register_dim(layout, "Y", 9);
    pdal_point_layout_register_dim(layout, "Z", 9);
    pdal_point_view_t* rview = pdal_point_view_create(layout);
    pdal_point_view_add_point(rview);
    pdal_point_view_set_f64(rview, 0, "X", 1406018.497);
    pdal_point_view_set_f64(rview, 0, "Y", 4917487.174);
    pdal_point_view_set_f64(rview, 0, "Z", 62.276);

    pdal_options_t* writeOpts = pdal_options_create();
    pdal_options_add_str(writeOpts, "filename", filename.c_str());
    pdal_options_add_str(writeOpts, "offset_x", "1000000");
    pdal_options_add_str(writeOpts, "scale_x", "0.001");
    pdal_options_add_str(writeOpts, "offset_y", "5000000");
    pdal_options_add_str(writeOpts, "scale_y", "0.001");
    pdal_options_add_str(writeOpts, "offset_z", "0");
    pdal_options_add_str(writeOpts, "scale_z", "0.001");
    pdal_writer_t* writer = pdal_writer_create_copc(writeOpts);
    EXPECT_NE(writer, nullptr);
    EXPECT_TRUE(pdal_writer_write_view(writer, rview));
    pdal_writer_destroy(writer);
    pdal_options_destroy(writeOpts);
    pdal_point_view_destroy(rview);

    // Read back via the C++ LasReader (which is Rust C ABI-backed for local
    // LAS/LAZ) and verify scale/offset preserved the input precision.
    Options readerOps;
    readerOps.add("filename", filename);

    PointTable readTable;

    LasReader reader;
    reader.setOptions(readerOps);

    reader.prepare(readTable);
    PointViewSet viewSet = reader.execute(readTable);
    EXPECT_EQ(viewSet.size(), 1u);
    PointViewPtr view = *viewSet.begin();
    EXPECT_EQ(view->size(), 1u);
    EXPECT_NEAR(1406018.497, view->getFieldAs<double>(Id::X, 0), .00001);
    EXPECT_NEAR(4917487.174, view->getFieldAs<double>(Id::Y, 0), .00001);
    EXPECT_NEAR(62.276, view->getFieldAs<double>(Id::Z, 0), .00001);
    FileUtils::deleteFile(filename);
}

TEST(CopcWriterTest, extradim)
{
    std::string outFilename(Support::temppath("copcdims.copc.laz"));

    FileUtils::deleteFile(outFilename);

    auto createFile = [&](const std::string& extraDims)
    {
        pdal_point_layout_t* layout = pdal_point_layout_create();
        pdal_point_layout_register_dim(layout, "X", 9);
        pdal_point_layout_register_dim(layout, "Y", 9);
        pdal_point_layout_register_dim(layout, "Z", 9);
        pdal_point_layout_register_dim(layout, "Q", 9);
        pdal_point_layout_register_dim(layout, "R", 9);
        pdal_point_layout_register_dim(layout, "S", 9);
        pdal_point_view_t* view = pdal_point_view_create(layout);
        pdal_point_view_add_point(view);
        pdal_point_view_set_f64(view, 0, "X", 1);
        pdal_point_view_set_f64(view, 0, "Y", 2);
        pdal_point_view_set_f64(view, 0, "Z", 3);
        pdal_point_view_set_f64(view, 0, "Q", 4);
        pdal_point_view_set_f64(view, 0, "R", 5);
        pdal_point_view_set_f64(view, 0, "S", 6);

        pdal_options_t* options = pdal_options_create();
        pdal_options_add_str(options, "filename", outFilename.c_str());
        pdal_options_add_str(options, "extra_dims", extraDims.c_str());
        pdal_writer_t* writer = pdal_writer_create_copc(options);
        if (!writer)
        {
            pdal_options_destroy(options);
            pdal_point_view_destroy(view);
            throw pdal_error(pdal_last_error() ? pdal_last_error()
                                               : "Rust COPC writer failed");
        }
        bool ok = pdal_writer_write_view(writer, view);
        pdal_writer_destroy(writer);
        pdal_options_destroy(options);
        pdal_point_view_destroy(view);
        if (!ok)
            throw pdal_error(pdal_last_error() ? pdal_last_error()
                                               : "Rust COPC writer failed");
    };

    auto verifyFile = [&](bool q, bool r, bool s) -> bool
    {
        LasReader r2;
        Options r2o;
        r2o.add("filename", outFilename);

        r2.setOptions(r2o);

        PointTable t2;
        r2.prepare(t2);
        PointLayoutPtr layout = t2.layout();

        FileUtils::deleteFile(outFilename);
        return ((q == (layout->findDim("Q") != Dimension::Id::Unknown)) &&
                (r == (layout->findDim("R") != Dimension::Id::Unknown)) &&
                (s == (layout->findDim("S") != Dimension::Id::Unknown)));
    };

    createFile("Q=int32"); // Q, but no R, S
    EXPECT_TRUE(verifyFile(true, false, false));
    createFile("all"); // Q, R, and S
    EXPECT_TRUE(verifyFile(true, true, true));
    createFile("Q=int32, S=double"); // Q, and S, no R
    EXPECT_TRUE(verifyFile(true, false, true));

    EXPECT_THROW(createFile("Q=int32, S"), pdal_error); // No type for S
    EXPECT_THROW(createFile("X=int32"), pdal_error);    // Existing dimension.
    EXPECT_THROW(createFile("Z=int32"), pdal_error);    // Unknown dimension.
}

} // namespace pdal
