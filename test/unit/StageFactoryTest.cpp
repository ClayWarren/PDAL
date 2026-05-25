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

#include <pdal/pdal_test_main.hpp>

#include <pdal/Log.hpp>
#include <pdal/PluginManager.hpp>
#include <pdal/StageExtensions.hpp>
#include <pdal/StageFactory.hpp>
#include <pdal/util/Algorithm.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

#include "Support.hpp"

namespace pdal
{

TEST(StageFactoryTest, Load)
{
    auto takeString = [](char* value)
    {
        std::string output(value ? value : "");
        pdal_string_free(value);
        return output;
    };

    const std::string stages = takeString(pdal_rust_stage_list_json());
    ASSERT_NE(stages.find("\"filters.crop\""), std::string::npos);
    ASSERT_NE(stages.find("\"readers.las\""), std::string::npos);
    ASSERT_NE(stages.find("\"writers.bpf\""), std::string::npos);
}

TEST(StageFactoryTest, extensionTest)
{
    EXPECT_EQ(StageFactory::inferWriterDriver("foo.laz"), "writers.las");
    EXPECT_EQ(StageFactory::inferWriterDriver("foo.las"), "writers.las");
    EXPECT_EQ(StageFactory::inferWriterDriver("STDOUT"), "writers.text");
    EXPECT_EQ(StageFactory::inferWriterDriver(""), "writers.text");
    EXPECT_EQ(StageFactory::inferWriterDriver("foo.tif"), "writers.gdal");
    EXPECT_EQ(StageFactory::inferWriterDriver("foo.tiff"), "writers.gdal");
    EXPECT_EQ(StageFactory::inferWriterDriver("foo.vrt"), "writers.gdal");

    EXPECT_EQ(StageFactory::inferReaderDriver("foo.laz"), "readers.las");
    EXPECT_EQ(StageFactory::inferReaderDriver("foo.las"), "readers.las");
    EXPECT_EQ(StageFactory::inferReaderDriver("http://foo.laz"), "readers.las");

    EXPECT_EQ(StageFactory::inferReaderDriver("foo.ntf"), "readers.nitf");
    EXPECT_EQ(StageFactory::inferWriterDriver("foo.ntf"), "writers.nitf");
    EXPECT_EQ(StageFactory::inferWriterDriver("junk.junk"), "");
}

TEST(StageFactoryTest, stageExtensionsLoadPerInstance)
{
    StageExtensions first{LogPtr()};
    EXPECT_EQ(first.defaultReader("pcd"), "readers.pcd");

    StageExtensions second{LogPtr()};
    EXPECT_EQ(second.defaultReader("pcd"), "readers.pcd");
    EXPECT_EQ(second.defaultWriter("pcd"), "writers.pcd");
}

TEST(StageFactoryTest, stageExtensionsCustomMappingsOverrideDefaults)
{
    auto takeString = [](char* value)
    {
        std::string output(value ? value : "");
        pdal_string_free(value);
        return output;
    };

    pdal_stage_extensions_t* extensions = pdal_stage_extensions_create();
    ASSERT_NE(extensions, nullptr);

    const char* readerExts[] = {"pcd", "customreader"};
    pdal_stage_extensions_set(extensions, "readers.custom", readerExts, 2);
    const char* writerExts[] = {"pcd", "customwriter"};
    pdal_stage_extensions_set(extensions, "writers.custom", writerExts, 2);

    EXPECT_EQ(
        takeString(pdal_stage_extensions_default_reader(extensions, "pcd")),
        "readers.custom");
    EXPECT_EQ(takeString(pdal_stage_extensions_default_reader(extensions,
                                                              "customreader")),
              "readers.custom");
    EXPECT_EQ(
        takeString(pdal_stage_extensions_default_writer(extensions, "pcd")),
        "writers.custom");
    EXPECT_EQ(takeString(pdal_stage_extensions_default_writer(extensions,
                                                              "customwriter")),
              "writers.custom");

    pdal_stage_extensions_destroy(extensions);
}

} // namespace pdal
