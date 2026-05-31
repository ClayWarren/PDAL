/******************************************************************************
 * Copyright (c) 2013, Howard Butler (hobu.inc@gmail.com)
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

#include <filters/CropFilter.hpp>
#include <io/LasReader.hpp>

#include <pdal/PointView.hpp>
#include <pdal/SpatialReference.hpp>
#include <pdal/StageFactory.hpp>
#include <pdal/util/FileUtils.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

#include <fstream>
#include <sstream>

#include <nlohmann/json.hpp>

#include "Support.hpp"
#include "filters/HexBinFilter.hpp"

using namespace pdal;

std::string takeRustString(char* value)
{
    std::string out(value ? value : "");
    pdal_string_free(value);
    return out;
}

void printChildren(std::ostream& out, MetadataNode m, int depth = 0)
{
    std::vector<MetadataNode> children = m.children();
    for (auto mi = children.begin(); mi != children.end(); ++mi)
    {
        MetadataNode& c = *mi;
        for (int i = 0; i < depth; i++)
            out << "\t";
        out << c.name() << " : " << c.value() << "\n";
        printChildren(out, c, depth + 1);
    }
}

TEST(HexbinFilterTest, HexbinFilterTest_test_1)
{
    StageFactory f;

    Options options;
    options.add("filename", Support::datapath("las/hextest.las"));

    Stage* reader(f.createStage("readers.las"));
    EXPECT_TRUE(reader);
    reader->setOptions(options);

    Stage* hexbin(f.createStage("filters.hexbin"));

    Options hexOptions;
    hexOptions.add("output_tesselation", true);
    hexOptions.add("sample_size", 5000);
    hexOptions.add("threshold", 1);
    hexOptions.add("edge_length", 0.666666666);
    EXPECT_TRUE(hexbin);
    hexbin->setOptions(hexOptions);
    hexbin->setInput(*reader);

    PointTable table;

    hexbin->prepare(table);
    hexbin->execute(table);

    MetadataNode m = table.metadata();
    m = m.findChild(hexbin->getName());

    std::string filename = Support::temppath("hexbin.txt");
    std::ofstream out(filename);
    printChildren(out, m);
    out.close();
    FileUtils::deleteFile(filename);
}

namespace
{
nlohmann::json readJsonFile(const std::string& path)
{
    std::ifstream in(path);
    std::stringstream ss;
    ss << in.rdbuf();
    return nlohmann::json::parse(ss.str());
}
} // namespace

// Gates the Rust-backed GeoJSON density/boundary file-output path. We assert
// the feature/field/geometry shape rather than exact coordinates so the test
// stays focused on the C ABI output contract.
TEST(HexbinFilterTest, ogr_density_boundary_output)
{
    StageFactory f;

    Options options;
    options.add("filename", Support::datapath("las/hextest.las"));

    Stage* reader(f.createStage("readers.las"));
    ASSERT_TRUE(reader);
    reader->setOptions(options);

    std::string densityFile = Support::temppath("hexbin_density.json");
    std::string boundaryFile = Support::temppath("hexbin_boundary.json");
    FileUtils::deleteFile(densityFile);
    FileUtils::deleteFile(boundaryFile);

    Stage* hexbin(f.createStage("filters.hexbin"));
    ASSERT_TRUE(hexbin);

    Options hexOptions;
    hexOptions.add("threshold", 1);
    hexOptions.add("edge_length", 0.666666666);
    hexOptions.add("density", densityFile);
    hexOptions.add("boundary", boundaryFile);
    hexOptions.add("ogrdriver", "GeoJSON");
    hexbin->setOptions(hexOptions);
    hexbin->setInput(*reader);

    PointTable table;
    hexbin->prepare(table);
    hexbin->execute(table);

    // Density: one Polygon feature per dense hexagon, with ID + COUNT fields
    // and a closed 7-vertex hexagonal ring (6 corners + repeated first vertex).
    nlohmann::json density = readJsonFile(densityFile);
    EXPECT_EQ(density["type"], "FeatureCollection");
    const auto& dfeatures = density["features"];
    ASSERT_TRUE(dfeatures.is_array());
    EXPECT_GT(dfeatures.size(), 0u);
    for (const auto& feature : dfeatures)
    {
        EXPECT_EQ(feature["geometry"]["type"], "Polygon");
        ASSERT_TRUE(feature["properties"].contains("ID"));
        ASSERT_TRUE(feature["properties"].contains("COUNT"));
        EXPECT_GE(feature["properties"]["COUNT"].get<int>(), 1);
        const auto& ring = feature["geometry"]["coordinates"][0];
        EXPECT_EQ(ring.size(), 7u);
    }

    // Boundary: a single MultiPolygon feature with ID == 0.
    nlohmann::json boundary = readJsonFile(boundaryFile);
    EXPECT_EQ(boundary["type"], "FeatureCollection");
    const auto& bfeatures = boundary["features"];
    ASSERT_TRUE(bfeatures.is_array());
    ASSERT_EQ(bfeatures.size(), 1u);
    EXPECT_EQ(bfeatures[0]["geometry"]["type"], "MultiPolygon");
    EXPECT_EQ(bfeatures[0]["properties"]["ID"].get<int>(), 0);

    FileUtils::deleteFile(densityFile);
    FileUtils::deleteFile(boundaryFile);
}

// testing sample size for calculating grid size
TEST(HexbinFilterTest, HexbinFilterTest_test_2)
{
    LasReader reader;
    Options readOpts;
    // Using a file with less points than the default sample size
    // (10 pts vs default 5000 sample size)
    readOpts.add("filename", Support::datapath("las/test_epsg_4326.las"));
    reader.setOptions(readOpts);

    HexBin filter;
    Options hexOpts;
    hexOpts.add("h3_grid", true);
    hexOpts.add("threshold", 1);
    filter.setOptions(hexOpts);
    filter.setInput(reader);

    PointTable table;
    filter.prepare(table);
    PointViewSet viewSet = filter.execute(table);

    MetadataNode m = table.metadata();
    m = m.findChild(filter.getName());
    // The H3 grid now runs through the Rust hexbin engine, which reports the
    // effective sample size (clamped to the point count) and the auto-estimated
    // H3 resolution via metadata rather than a C++ grid object.
    EXPECT_EQ(m.findChild("sample_size").value<int>(), 10);
    EXPECT_EQ(m.findChild("h3_resolution").value<int>(), 13);

    // Now testing with a non-h3 grid
    HexBin filter2;
    Options hexOpts2;
    hexOpts2.add("h3_grid", false);
    hexOpts2.add("threshold", 1);
    filter2.setOptions(hexOpts2);
    filter2.setInput(reader);

    PointTable table2;
    filter2.prepare(table2);
    PointViewSet viewSet2 = filter2.execute(table2);

    MetadataNode m2 = table2.metadata();
    m2 = m2.findChild(filter2.getName());

    // The non-H3 standard grid now runs through the Rust hexbin engine, which
    // reports the effective sample size (clamped to the point count) and the
    // estimated edge length via metadata rather than a C++ grid object.
    EXPECT_EQ(m2.findChild("sample_size").value<int>(), 10);
    EXPECT_FLOAT_EQ(m2.findChild("estimated_edge").value<float>(), 1e-05);
}

// Test that we create proper WKT for geometry with islands.
TEST(HexbinFilterTest, HexGrid_issue_2507)
{
    std::vector<int32_t> hexes = {
        0, 3, 0, 4, 0, 5, 0, 6, 1, 2, 1, 6, 2, 2, 2, 4, 2, 5, 2,
        7, 3, 1, 3, 3, 3, 5, 3, 7, 4, 1, 4, 2, 4, 4, 4, 5, 4, 8,
        5, 0, 5, 2, 5, 6, 5, 8, 6, 1, 6, 3, 6, 4, 6, 8, 7, 1, 7,
        3, 7, 4, 7, 5, 7, 7, 8, 2, 8, 3, 8, 4, 8, 5, 8, 6, 8, 7};
    std::string s = takeRustString(
        pdal_hexgrid_wkt(1.0, 1, hexes.data(), hexes.size() / 2, 6));

    std::string test =
        R"delim(MULTIPOLYGON (((4.90748 0.5, 5.19615 1, 5.7735 1, 6.06218 1.5, 6.63953 1.5, 6.9282 2, 7.50555 2, 7.79423 2.5, 7.50555 3, 7.79423 3.5, 7.50555 4, 7.79423 4.5, 7.50555 5, 7.79423 5.5, 7.50555 6, 7.79423 6.5, 7.50555 7, 7.79423 7.5, 7.50555 8, 6.9282 8, 6.63953 8.5, 6.06218 8.5, 5.7735 9, 5.19615 9, 4.90748 9.5, 4.33013 9.5, 4.04145 9, 3.4641 9, 3.17543 8.5, 2.59808 8.5, 2.3094 8, 1.73205 8, 1.44338 7.5, 0.866025 7.5, 0.57735 7, 0 7, -0.288675 6.5, 0 6, -0.288675 5.5, 0 5, -0.288675 4.5, 0 4, -0.288675 3.5, 0 3, 0.57735 3, 0.866025 2.5, 1.44338 2.5, 1.73205 2, 2.3094 2, 2.59808 1.5, 3.17543 1.5, 3.4641 1, 4.04145 1, 4.33013 0.5, 4.90748 0.5), (4.90748 2.5, 4.33013 2.5, 4.04145 2, 4.33013 1.5, 4.90748 1.5, 5.19615 2, 5.7735 2, 6.06218 2.5, 6.63953 2.5, 6.9282 3, 6.63953 3.5, 6.06218 3.5, 5.7735 3, 5.19615 3, 4.90748 2.5), (1.44338 6.5, 0.866025 6.5, 0.57735 6, 0.866025 5.5, 0.57735 5, 0.866025 4.5, 0.57735 4, 0.866025 3.5, 1.44338 3.5, 1.73205 3, 2.3094 3, 2.59808 2.5, 3.17543 2.5, 3.4641 3, 4.04145 3, 4.33013 3.5, 4.90748 3.5, 5.19615 4, 4.90748 4.5, 5.19615 5, 5.7735 5, 6.06218 5.5, 5.7735 6, 6.06218 6.5, 6.63953 6.5, 6.9282 7, 6.63953 7.5, 6.06218 7.5, 5.7735 8, 5.19615 8, 4.90748 8.5, 4.33013 8.5, 4.04145 8, 3.4641 8, 3.17543 7.5, 2.59808 7.5, 2.3094 7, 1.73205 7, 1.44338 6.5)), ((3.17543 3.5, 3.4641 4, 4.04145 4, 4.33013 4.5, 4.04145 5, 4.33013 5.5, 4.04145 6, 3.4641 6, 3.17543 6.5, 2.59808 6.5, 2.3094 6, 1.73205 6, 1.44338 5.5, 1.73205 5, 1.44338 4.5, 1.73205 4, 2.3094 4, 2.59808 3.5, 3.17543 3.5), (3.17543 5.5, 2.59808 5.5, 2.3094 5, 2.59808 4.5, 3.17543 4.5, 3.4641 5, 3.17543 5.5)), ((4.90748 6.5, 5.19615 7, 4.90748 7.5, 4.33013 7.5, 4.04145 7, 4.33013 6.5, 4.90748 6.5))))delim";
    EXPECT_EQ(s, test);
}

TEST(HexbinFilterTest, H3Grid_issue_2507)
{
    std::vector<int32_t> hexes = {
        5, 2, 5, 3, 6, 2, 6, 4, 7,  3, 7,  4, 3,  0, 3,  1, 3,  2, 3,  3, 3, 4,
        3, 5, 4, 0, 4, 4, 4, 6, 5,  0, 5,  2, 5,  3, 5,  5, 5,  7, 6,  0, 6, 2,
        6, 4, 6, 6, 6, 8, 7, 1, 7,  3, 7,  4, 7,  7, 7,  8, 8,  2, 8,  7, 8, 8,
        9, 3, 9, 5, 9, 7, 9, 8, 10, 4, 10, 8, 11, 5, 11, 6, 11, 7, 11, 8,
    };
    std::string s = takeRustString(pdal_h3grid_wkt(
        10, 1, 40.689167, -74.044444, hexes.data(), hexes.size() / 2, 6));

    std::string test =
        R"delim(MULTIPOLYGON (((-70.1413 39.9976, -70.1407 39.9971, -70.141 39.9965, -70.1419 39.9964, -70.1423 39.9958, -70.1432 39.9957, -70.1435 39.995, -70.1444 39.9949, -70.1447 39.9943, -70.1456 39.9942, -70.1462 39.9947, -70.1471 39.9946, -70.1477 39.9951, -70.1486 39.9949, -70.1492 39.9954, -70.1501 39.9953, -70.1507 39.9958, -70.1516 39.9957, -70.1521 39.9962, -70.153 39.9961, -70.1536 39.9966, -70.1533 39.9972, -70.1539 39.9977, -70.1535 39.9983, -70.1541 39.9988, -70.1538 39.9994, -70.1544 39.9999, -70.154 40.0006, -70.1531 40.0007, -70.1528 40.0013, -70.1519 40.0014, -70.1516 40.002, -70.1507 40.0022, -70.1504 40.0028, -70.1495 40.0029, -70.1491 40.0035, -70.1482 40.0036, -70.1479 40.0043, -70.147 40.0044, -70.1464 40.0039, -70.1455 40.004, -70.1449 40.0035, -70.144 40.0036, -70.1435 40.0031, -70.1426 40.0032, -70.142 40.0027, -70.1423 40.0021, -70.1417 40.0016, -70.1421 40.001, -70.1415 40.0005, -70.1418 39.9999, -70.1412 39.9994, -70.1415 39.9988, -70.141 39.9983, -70.1413 39.9976), (-70.1465 39.9958, -70.1474 39.9957, -70.1479 39.9962, -70.1488 39.9961, -70.1494 39.9966, -70.1491 39.9972, -70.1497 39.9977, -70.1494 39.9983, -70.1499 39.9988, -70.1496 39.9994, -70.1502 39.9999, -70.1499 40.0005, -70.149 40.0007, -70.1486 40.0013, -70.1477 40.0014, -70.1474 40.002, -70.148 40.0025, -70.1477 40.0031, -70.1467 40.0033, -70.1462 40.0028, -70.1453 40.0029, -70.1447 40.0024, -70.1438 40.0025, -70.1432 40.002, -70.1435 40.0014, -70.143 40.0009, -70.1433 40.0003, -70.1427 39.9998, -70.143 39.9991, -70.1425 39.9986, -70.1428 39.998, -70.1422 39.9975, -70.1425 39.9969, -70.1434 39.9968, -70.1438 39.9962, -70.1447 39.996, -70.145 39.9954, -70.1459 39.9953, -70.1465 39.9958), (-70.1524 39.9973, -70.1521 39.9979, -70.1526 39.9984, -70.1523 39.9991, -70.1529 39.9996, -70.1526 40.0002, -70.1517 40.0003, -70.1511 39.9998, -70.1514 39.9992, -70.1508 39.9987, -70.1512 39.9981, -70.1506 39.9976, -70.1509 39.9969, -70.1518 39.9968, -70.1524 39.9973)), ((-70.1443 39.9984, -70.1437 39.9979, -70.144 39.9973, -70.1449 39.9972, -70.1452 39.9965, -70.1461 39.9964, -70.1467 39.9969, -70.1476 39.9968, -70.1482 39.9973, -70.1479 39.9979, -70.1484 39.9984, -70.1481 39.999, -70.1472 39.9992, -70.1469 39.9998, -70.146 39.9999, -70.1454 39.9994, -70.1445 39.9995, -70.1439 39.999, -70.1443 39.9984), (-70.147 39.998, -70.1466 39.9987, -70.1457 39.9988, -70.1452 39.9983, -70.1455 39.9977, -70.1464 39.9975, -70.147 39.998)), ((-70.145 40.0018, -70.1444 40.0013, -70.1448 40.0006, -70.1457 40.0005, -70.1462 40.001, -70.1459 40.0016, -70.145 40.0018))))delim";
    EXPECT_EQ(s, test);
}

// Checks that boundary vertices are correct for a smoothed hexbin
TEST(HexbinFilterTest, issue_4899)
{
    LasReader r;
    Options ro;
    ro.add("filename", Support::datapath("filters/hexbin-crop.las"));
    r.setOptions(ro);

    // running with defaults here (smooth=true)
    HexBin h;
    h.setInput(r);
    PointTable table;
    h.prepare(table);
    PointViewSet viewSet = h.execute(table);
    PointViewPtr view = *viewSet.begin();

    MetadataNode m = table.metadata();
    m = m.findChild(h.getName());
    std::string wkt = m.findChild("boundary").value();

    std::string test =
        R"delim(POLYGON ((394125.55 3688517.9,394293.29 3689389.5,393790.06 3689680.0,393119.1 3689098.9,394125.55 3688517.9)))delim";

    // probably not enough precision to be equal. Implementing
    // Polygon::equal would fix, but not sure why it hasn't been already.
    EXPECT_EQ(wkt, test);
}
