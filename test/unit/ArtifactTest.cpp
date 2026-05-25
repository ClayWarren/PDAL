/******************************************************************************
 * Copyright (c) 2016, Howard Butler <howard@hobu.co>
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
 *       notice, this list of conditions and the following disclaimer in the
 *       documentation and/or other materials provided with the distribution.
 *     * Neither the name of Hobu, Inc. nor the names of its contributors may
 *       be used to endorse or promote products derived from this software
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
 * LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 ****************************************************************************/

#include <pdal/pdal_test_main.hpp>

#include <rust/pdal-capi/include/pdal_capi.h>

#include <algorithm>
#include <string>
#include <vector>
#include <vendor/nlohmann/nlohmann/json.hpp>

namespace pdal
{

namespace
{

constexpr const char* TestArtifact = "TestArtifact";
constexpr const char* TestArtifact2 = "TestArtifact2";
constexpr const char* Foo = "Foo";

std::string takeString(char* raw)
{
    if (!raw)
        return std::string();
    std::string out(raw);
    pdal_string_free(raw);
    return out;
}

std::vector<std::string> keys(pdal_artifact_manager_t* manager)
{
    return NL::json::parse(takeString(pdal_artifact_manager_keys_json(manager)))
        .get<std::vector<std::string>>();
}

} // namespace

TEST(ArtifactTest, simple)
{
    pdal_artifact_manager_t* t = pdal_artifact_manager_create();
    EXPECT_TRUE(pdal_artifact_manager_put(t, "MyTest", TestArtifact, "MyTest"));
    EXPECT_EQ(pdal_artifact_manager_get(t, "foo", TestArtifact), nullptr);
    EXPECT_EQ(takeString(pdal_artifact_manager_get(t, "MyTest", TestArtifact)),
              "MyTest");
    EXPECT_EQ(pdal_artifact_manager_get(t, "MyTest", Foo), nullptr);
    EXPECT_EQ(pdal_artifact_manager_get(t, "MyTest", TestArtifact2), nullptr);
    pdal_artifact_manager_destroy(t);
}

TEST(ArtifactTest, replace)
{
    pdal_artifact_manager_t* t = pdal_artifact_manager_create();
    EXPECT_FALSE(pdal_artifact_manager_exists(t, "MyTest"));
    EXPECT_FALSE(
        pdal_artifact_manager_replace(t, "MyTest", TestArtifact, "MyTest"));
    EXPECT_TRUE(pdal_artifact_manager_put(t, "MyTest", TestArtifact, "MyTest"));
    EXPECT_FALSE(pdal_artifact_manager_replace(t, "MyTest", TestArtifact2, ""));
    EXPECT_TRUE(
        pdal_artifact_manager_replace(t, "MyTest", TestArtifact, "MyTestA"));
    EXPECT_TRUE(pdal_artifact_manager_exists(t, "MyTest"));
    EXPECT_EQ(takeString(pdal_artifact_manager_get(t, "MyTest", TestArtifact)),
              "MyTestA");
    EXPECT_FALSE(pdal_artifact_manager_erase(t, "MyOtherTest"));
    EXPECT_TRUE(pdal_artifact_manager_erase(t, "MyTest"));
    EXPECT_FALSE(pdal_artifact_manager_exists(t, "MyTest"));
    pdal_artifact_manager_destroy(t);
}

TEST(ArtifactTest, replaceOrPut)
{
    pdal_artifact_manager_t* t = pdal_artifact_manager_create();
    EXPECT_FALSE(pdal_artifact_manager_exists(t, "MyTest"));
    EXPECT_TRUE(pdal_artifact_manager_replace_or_put(t, "MyTest", TestArtifact,
                                                     "MyTest"));
    EXPECT_EQ(takeString(pdal_artifact_manager_get(t, "MyTest", TestArtifact)),
              "MyTest");
    EXPECT_TRUE(pdal_artifact_manager_exists(t, "MyTest"));
    EXPECT_TRUE(pdal_artifact_manager_replace_or_put(t, "MyTest", TestArtifact,
                                                     "MyTestA"));
    EXPECT_TRUE(pdal_artifact_manager_exists(t, "MyTest"));
    EXPECT_EQ(takeString(pdal_artifact_manager_get(t, "MyTest", TestArtifact)),
              "MyTestA");
    EXPECT_FALSE(
        pdal_artifact_manager_replace_or_put(t, "MyTest", TestArtifact2, ""));
    pdal_artifact_manager_destroy(t);
}

TEST(ArtifactTest, key_access)
{
    pdal_artifact_manager_t* t = pdal_artifact_manager_create();
    EXPECT_TRUE(keys(t).empty());

    EXPECT_TRUE(pdal_artifact_manager_put(t, "MyTest", TestArtifact, "MyTest"));
    EXPECT_EQ(keys(t).size(), 1U);
    EXPECT_EQ(keys(t).at(0), "MyTest");

    EXPECT_TRUE(
        pdal_artifact_manager_put(t, "MyTest2", TestArtifact, "MyTest"));
    std::vector<std::string> ks = keys(t);
    EXPECT_EQ(ks.size(), 2U);
    EXPECT_EQ(std::find(ks.begin(), ks.end(), "Foo"), ks.end());
    EXPECT_NE(std::find(ks.begin(), ks.end(), "MyTest"), ks.end());
    EXPECT_NE(std::find(ks.begin(), ks.end(), "MyTest2"), ks.end());
    pdal_artifact_manager_destroy(t);
}

} // namespace pdal
