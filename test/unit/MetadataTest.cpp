/******************************************************************************
 * Copyright (c) 2011, Michael P. Gerlek (mpg@flaxen.com)
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

#include <pdal/Metadata.hpp>
#include <pdal/PDALUtils.hpp>
#include <pdal/SpatialReference.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

using namespace pdal;

namespace
{
std::string takeString(char* value)
{
    if (!value)
        return "";
    std::string output(value);
    pdal_string_free(value);
    return output;
}
} // namespace

TEST(MetadataTest, assign)
{
    pdal_metadata_node_t* m1 = pdal_metadata_node_create("Test");
    pdal_metadata_node_t* m2 = pdal_metadata_node_clone(m1);
    EXPECT_EQ(takeString(pdal_metadata_node_name(m1)), "Test");
    EXPECT_EQ(takeString(pdal_metadata_node_name(m2)), "Test");
    pdal_metadata_node_destroy(m2);
    pdal_metadata_node_destroy(m1);
}

TEST(MetadataTest, test_construction)
{
    uint32_t u32(32u);
    int32_t i32(-32);
    uint64_t u64(64u);
    int64_t i64(-64);
    int8_t i8(-8);
    uint8_t u8(8);
    int16_t i16(-16);
    uint16_t u16(16);

    {
        std::vector<uint8_t> v;
        v.reserve(100);
        for (uint8_t i = 0; i < 100; i++)
            v.push_back(i);

        pdal_metadata_node_t* m = pdal_metadata_node_create("name");
        pdal_metadata_node_set_type(m, "base64Binary");
        pdal_metadata_node_set_string(
            m,
            takeString(pdal_utils_base64_encode(v.data(), v.size())).c_str());
        std::string base64(
            "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMD"
            "EyMzQ1Njc4OTo7PD0+"
            "P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiYw==");
        EXPECT_EQ(takeString(pdal_metadata_node_type(m)), "base64Binary");
        EXPECT_EQ(takeString(pdal_metadata_node_value(m)), base64);
        pdal_metadata_node_destroy(m);
    }

    {
        pdal_metadata_node_t* m = pdal_metadata_node_create("name");
        pdal_metadata_node_set_i64(m, i8);
        pdal_metadata_node_set_type(m, "integer");
        EXPECT_EQ(takeString(pdal_metadata_node_value(m)), "-8");
        EXPECT_EQ(takeString(pdal_metadata_node_type(m)), "integer");
        pdal_metadata_node_destroy(m);
    }

    {
        pdal_metadata_node_t* m = pdal_metadata_node_create("name");
        pdal_metadata_node_set_i64(m, i16);
        pdal_metadata_node_set_type(m, "integer");
        EXPECT_EQ(takeString(pdal_metadata_node_value(m)), "-16");
        EXPECT_EQ(takeString(pdal_metadata_node_type(m)), "integer");
        pdal_metadata_node_destroy(m);
    }

    {
        pdal_metadata_node_t* m = pdal_metadata_node_create("name");
        pdal_metadata_node_set_i64(m, i32);
        pdal_metadata_node_set_type(m, "integer");
        EXPECT_EQ(takeString(pdal_metadata_node_value(m)), "-32");
        EXPECT_EQ(takeString(pdal_metadata_node_type(m)), "integer");
        pdal_metadata_node_destroy(m);
    }

    {
        pdal_metadata_node_t* m = pdal_metadata_node_create("name");
        pdal_metadata_node_set_i64(m, i64);
        pdal_metadata_node_set_type(m, "integer");
        EXPECT_EQ(takeString(pdal_metadata_node_value(m)), "-64");
        EXPECT_EQ(takeString(pdal_metadata_node_type(m)), "integer");
        pdal_metadata_node_destroy(m);
    }

    {
        pdal_metadata_node_t* m = pdal_metadata_node_create("name");
        pdal_metadata_node_set_i64(m, i64);
        pdal_metadata_node_set_type(m, "integer");
        EXPECT_EQ(takeString(pdal_metadata_node_value(m)), "-64");
        EXPECT_EQ(takeString(pdal_metadata_node_type(m)), "integer");
        pdal_metadata_node_destroy(m);
    }

    {
        pdal_metadata_node_t* m = pdal_metadata_node_create("name");
        pdal_metadata_node_set_u64(m, u8);
        pdal_metadata_node_set_type(m, "nonNegativeInteger");
        EXPECT_EQ(takeString(pdal_metadata_node_value(m)), "8");
        EXPECT_EQ(takeString(pdal_metadata_node_type(m)), "nonNegativeInteger");
        pdal_metadata_node_destroy(m);
    }

    {
        pdal_metadata_node_t* m = pdal_metadata_node_create("name");
        pdal_metadata_node_set_u64(m, u16);
        pdal_metadata_node_set_type(m, "nonNegativeInteger");
        EXPECT_EQ(takeString(pdal_metadata_node_value(m)), "16");
        EXPECT_EQ(takeString(pdal_metadata_node_type(m)), "nonNegativeInteger");
        pdal_metadata_node_destroy(m);
    }

    {
        pdal_metadata_node_t* m = pdal_metadata_node_create("name");
        pdal_metadata_node_set_u64(m, u32);
        pdal_metadata_node_set_type(m, "nonNegativeInteger");
        EXPECT_EQ(takeString(pdal_metadata_node_value(m)), "32");
        EXPECT_EQ(takeString(pdal_metadata_node_type(m)), "nonNegativeInteger");
        pdal_metadata_node_destroy(m);
    }

    {
        pdal_metadata_node_t* m = pdal_metadata_node_create("name");
        pdal_metadata_node_set_u64(m, u64);
        pdal_metadata_node_set_type(m, "nonNegativeInteger");
        EXPECT_EQ(takeString(pdal_metadata_node_value(m)), "64");
        EXPECT_EQ(takeString(pdal_metadata_node_type(m)), "nonNegativeInteger");
        pdal_metadata_node_destroy(m);
    }
}

TEST(MetadataTest, typed_value)
{
    pdal_metadata_node_t* m = pdal_metadata_node_create("name");
    pdal_metadata_node_set_i64(m, 127);

    EXPECT_EQ(127, pdal_metadata_node_value_i64(m));

    double d = 123.45;
    std::string encoded = takeString(
        pdal_utils_base64_encode(reinterpret_cast<uint8_t*>(&d), sizeof(d)));
    double decoded = 0;
    ASSERT_TRUE(
        pdal_metadata_value_as_f64("base64Binary", encoded.c_str(), &decoded));
    EXPECT_DOUBLE_EQ(d, decoded);
    EXPECT_EQ("zczMzMzcXkA=", encoded);

    uint64_t value = 0;
    ASSERT_TRUE(pdal_metadata_value_as_u64("string", "65539", &value));
    EXPECT_EQ(65539u, value);

    uint64_t invalid = 0;
    EXPECT_FALSE(
        pdal_metadata_value_as_u64("string", "not-a-number", &invalid));
    pdal_metadata_node_destroy(m);
}

TEST(MetadataTest, test_construction_with_srs)
{
    pdal_spatial_reference_t* ref = pdal_spatial_reference_create("EPSG:4326");
    pdal_metadata_node_t* m = pdal_spatial_reference_to_metadata(ref);
    ASSERT_NE(m, nullptr);
    EXPECT_EQ(takeString(pdal_metadata_node_name(m)), "srs");
    ASSERT_EQ(pdal_metadata_node_child_count(m), 1u);
    pdal_metadata_node_t* wkt = pdal_metadata_node_child(m, 0);
    ASSERT_NE(wkt, nullptr);
    EXPECT_EQ(takeString(pdal_metadata_node_name(wkt)), "wkt");
    EXPECT_EQ(takeString(pdal_metadata_node_value(wkt)), "EPSG:4326");
    pdal_metadata_node_destroy(wkt);
    pdal_metadata_node_destroy(m);
    pdal_spatial_reference_destroy(ref);
}

TEST(MetadataTest, test_metadata_copy)
{
    pdal_metadata_node_t* m = pdal_metadata_node_create("val");
    pdal_metadata_node_set_u64(m, 2);
    pdal_metadata_node_t* m2 = pdal_metadata_node_clone(m);
    EXPECT_EQ(pdal_metadata_node_value_u64(m2), 2u);
    pdal_metadata_node_destroy(m2);
    pdal_metadata_node_destroy(m);
}

TEST(MetadataTest, test_metadata_set)
{
    pdal_metadata_node_t* m = pdal_metadata_node_create("");
    pdal_metadata_node_t* m1 = pdal_metadata_node_create("m1");
    pdal_metadata_node_set_u64(m1, 1);
    pdal_metadata_node_t* m2 = pdal_metadata_node_create("m2");
    pdal_metadata_node_set_i64(m2, 2);
    pdal_metadata_node_t* m1prime = pdal_metadata_node_create("m1prime");
    pdal_metadata_node_set_string(m1prime, "Some other metadata");

    pdal_metadata_node_add_child(m, m1);
    pdal_metadata_node_add_child(m, m2);
    pdal_metadata_node_add_child(m, m1prime);

    EXPECT_EQ(pdal_metadata_node_child_count(m), 3u);
    pdal_metadata_node_t* node = pdal_metadata_node_child_named(m, "m1", 0);
    EXPECT_EQ(takeString(pdal_metadata_node_value(node)), "1");
    pdal_metadata_node_destroy(node);
    node = pdal_metadata_node_child_named(m, "m2", 0);
    EXPECT_EQ(takeString(pdal_metadata_node_value(node)), "2");
    pdal_metadata_node_destroy(node);
    node = pdal_metadata_node_child_named(m, "m1prime", 0);
    EXPECT_EQ(takeString(pdal_metadata_node_value(node)),
              "Some other metadata");
    pdal_metadata_node_destroy(node);
    EXPECT_EQ(pdal_metadata_node_child_named_count(m, "foo"), 0u);
    pdal_metadata_node_destroy(m);
}

TEST(MetadataTest, test_vlr_metadata)
{
    pdal_metadata_node_t* m = pdal_metadata_node_create("");

    pdal_metadata_node_t* bogusvlr = pdal_metadata_node_create("vlr1");
    pdal_metadata_node_set_string(bogusvlr, "VLR1VALUE");
    pdal_metadata_node_set_description(bogusvlr, "VLR1DESC");
    pdal_metadata_node_t* vlr = pdal_metadata_node_create("vlr2");
    pdal_metadata_node_set_string(vlr, "VLR2VALUE");
    pdal_metadata_node_set_description(vlr, "VLR2DESC");
    pdal_metadata_node_t* recordId = pdal_metadata_node_create("record_id");
    pdal_metadata_node_set_string(recordId, "MYRECOREDID");
    pdal_metadata_node_t* userId = pdal_metadata_node_create("user_id");
    pdal_metadata_node_set_string(userId, "MYUSERID");
    pdal_metadata_node_add_child(vlr, recordId);
    pdal_metadata_node_add_child(vlr, userId);
    pdal_metadata_node_add_child(m, bogusvlr);
    pdal_metadata_node_add_child(m, vlr);

    pdal_metadata_node_t* found = pdal_metadata_node_child_named(m, "vlr2", 0);
    ASSERT_NE(found, nullptr);
    EXPECT_EQ(takeString(pdal_metadata_node_name(found)), "vlr2");
    EXPECT_EQ(takeString(pdal_metadata_node_value(found)), "VLR2VALUE");
    EXPECT_EQ(takeString(pdal_metadata_node_description(found)), "VLR2DESC");
    EXPECT_EQ(pdal_metadata_node_child_named_count(found, "record_id"), 1u);
    EXPECT_EQ(pdal_metadata_node_child_named_count(found, "user_id"), 1u);
    pdal_metadata_node_destroy(found);
    pdal_metadata_node_destroy(m);
}

TEST(MetadataTest, find_child_string)
{
    pdal_metadata_node_t* top = pdal_metadata_node_create("");
    pdal_metadata_node_t* level1 = pdal_metadata_node_create("level1");
    pdal_metadata_node_t* level2 = pdal_metadata_node_create("level2");
    pdal_metadata_node_t* child = pdal_metadata_node_create("210");
    pdal_metadata_node_set_string(child, "210");
    pdal_metadata_node_add_child(level2, child);
    child = pdal_metadata_node_create("220");
    pdal_metadata_node_set_string(child, "220");
    pdal_metadata_node_add_child(level2, child);
    pdal_metadata_node_add_child(level1, level2);
    pdal_metadata_node_add_child(top, level1);

    pdal_metadata_node_t* n =
        pdal_metadata_node_find_child_path(top, "level1:level2:210");
    ASSERT_NE(n, nullptr);
    EXPECT_EQ(takeString(pdal_metadata_node_value(n)), "210");
    pdal_metadata_node_destroy(n);

    n = pdal_metadata_node_find_child_path(top, "level1:level2:220");
    ASSERT_NE(n, nullptr);
    EXPECT_EQ(takeString(pdal_metadata_node_value(n)), "220");
    pdal_metadata_node_destroy(n);
    pdal_metadata_node_destroy(top);
}

// Make sure that we handle double-precision values to 10 decimal places.
TEST(MetadataTest, test_float)
{
    pdal_metadata_node_t* n = pdal_metadata_node_create("top");
    pdal_metadata_node_set_f64(n, 1e-20);
    EXPECT_DOUBLE_EQ(pdal_metadata_node_value_f64(n), 1e-20);

    pdal_metadata_node_set_f64(n, 1.12345678);
    EXPECT_DOUBLE_EQ(pdal_metadata_node_value_f64(n), 1.12345678);
    pdal_metadata_node_destroy(n);
}

// Test that pointers traverse metadata.
TEST(MetadataTest, pointer)
{
    class foo
    {
    };

    foo f;

    MetadataNode n("top");
    MetadataNode n2 = n.add("test", &f);

    std::istringstream iss;
    foo* f2 = n2.value<foo*>();
    EXPECT_EQ(f2, &f);
}

// Test the output of infinity/nan
TEST(MetadataTest, infnan)
{
    EXPECT_EQ(takeString(pdal_metadata_json_value("double", "NaN")), "\"NaN\"");

    EXPECT_EQ(takeString(pdal_metadata_json_value("double", "Infinity")),
              "\"Infinity\"");

    EXPECT_EQ(takeString(pdal_metadata_json_value("double", "-Infinity")),
              "\"-Infinity\"");
}

// Test the addOrUpdate functions.
TEST(MetadataTest, update)
{
    pdal_metadata_node_t* root = pdal_metadata_node_create("root");

    EXPECT_EQ(pdal_metadata_node_child_count(root), 0u);
    pdal_metadata_node_t* child = pdal_metadata_node_create("test");
    pdal_metadata_node_set_i64(child, 21);
    pdal_metadata_node_add_or_update_child(root, child);
    EXPECT_EQ(pdal_metadata_node_child_count(root), 1u);

    pdal_metadata_node_t* replacement = pdal_metadata_node_create("test");
    pdal_metadata_node_set_i64(replacement, 22);
    pdal_metadata_node_set_description(replacement, "description");
    pdal_metadata_node_add_or_update_child(root, replacement);
    EXPECT_EQ(pdal_metadata_node_child_count(root), 1u);

    pdal_metadata_node_t* n = pdal_metadata_node_child(root, 0);
    ASSERT_NE(n, nullptr);
    EXPECT_EQ(takeString(pdal_metadata_node_name(n)), "test");
    EXPECT_EQ(takeString(pdal_metadata_node_description(n)), "description");
    EXPECT_EQ(pdal_metadata_node_value_i64(n), 22);
    pdal_metadata_node_destroy(n);

    pdal_metadata_node_t* root2 = pdal_metadata_node_create("root2");
    child = pdal_metadata_node_create("child");
    pdal_metadata_node_t* subchild = pdal_metadata_node_create("subchild1");
    pdal_metadata_node_set_i64(subchild, 1);
    pdal_metadata_node_add_child(child, subchild);
    subchild = pdal_metadata_node_create("subchild2");
    pdal_metadata_node_set_i64(subchild, 2);
    pdal_metadata_node_add_child(child, subchild);
    pdal_metadata_node_add_child(root2, child);

    replacement = pdal_metadata_node_create("child");
    for (int i = 3; i <= 5; ++i)
    {
        subchild =
            pdal_metadata_node_create(("subchild" + std::to_string(i)).c_str());
        pdal_metadata_node_set_i64(subchild, i);
        pdal_metadata_node_add_child(replacement, subchild);
    }
    pdal_metadata_node_add_or_update_child(root2, replacement);

    EXPECT_EQ(pdal_metadata_node_child_count(root2), 1u);
    n = pdal_metadata_node_child(root2, 0);
    ASSERT_NE(n, nullptr);
    EXPECT_EQ(pdal_metadata_node_child_count(n), 3u);
    for (uint64_t i = 0; i < 3; ++i)
    {
        pdal_metadata_node_t* grandchild = pdal_metadata_node_child(n, i);
        EXPECT_EQ(pdal_metadata_node_value_i64(grandchild),
                  static_cast<int64_t>(i + 3));
        pdal_metadata_node_destroy(grandchild);
    }
    pdal_metadata_node_destroy(n);
    pdal_metadata_node_destroy(root2);
    pdal_metadata_node_destroy(root);
}
