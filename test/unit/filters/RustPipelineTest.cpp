/******************************************************************************
 * Copyright (c) 2026, PDAL Authors
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

#include <io/FauxReader.hpp>

#include <pdal/private/Raster.hpp>

#include <filters/private/RustPipeline.hpp>
#include <filters/private/RustViewConverter.hpp>

using namespace pdal;

TEST(RustPipelineTest, singleStagePipeline)
{
    BOX3D srcBounds(0.0, 0.0, 1.0, 0.0, 0.0, 10.0);
    Options readerOps;
    readerOps.add("bounds", srcBounds);
    readerOps.add("mode", "ramp");
    readerOps.add("count", 10);

    FauxReader reader;
    reader.setOptions(readerOps);

    PointTable table;
    reader.prepare(table);
    PointViewSet readSet = reader.execute(table);
    PointViewPtr inputView = *readSet.begin();
    EXPECT_EQ(inputView->size(), 10u);

    pdal_options_t* ops = pdal_options_create();
    pdal_options_add_u64(ops, "count", 5);
    pdal_options_add_str(ops, "invert", "false");
    pdal_stage_t* headStage = pdal_stage_create_head(ops);
    pdal_options_destroy(ops);
    ASSERT_NE(headStage, nullptr);

    RustPipeline pipeline;
    int64_t idx = pipeline.addStage(headStage);
    EXPECT_GE(idx, 0);
    EXPECT_EQ(pipeline.stageCount(), 1u);

    PointViewPtr output = pipeline.execute(inputView);
    EXPECT_EQ(output->size(), 5u);

    for (PointId i = 0; i < output->size(); ++i)
    {
        double z = output->getFieldAs<double>(Dimension::Id::Z, i);
        EXPECT_EQ(static_cast<int>(z), static_cast<int>(i + 1));
    }
}

TEST(RustPipelineTest, linearPipelineTwoStages)
{
    BOX3D srcBounds(0.0, 0.0, 1.0, 0.0, 0.0, 30.0);
    Options readerOps;
    readerOps.add("bounds", srcBounds);
    readerOps.add("mode", "ramp");
    readerOps.add("count", 30);

    FauxReader reader;
    reader.setOptions(readerOps);

    PointTable table;
    reader.prepare(table);
    PointViewSet readSet = reader.execute(table);
    PointViewPtr inputView = *readSet.begin();
    EXPECT_EQ(inputView->size(), 30u);

    // head(10) -> tail(3) should yield points 8, 9, 10
    pdal_options_t* headOps = pdal_options_create();
    pdal_options_add_u64(headOps, "count", 10);
    pdal_options_add_str(headOps, "invert", "false");
    pdal_stage_t* headStage = pdal_stage_create_head(headOps);
    pdal_options_destroy(headOps);

    pdal_options_t* tailOps = pdal_options_create();
    pdal_options_add_u64(tailOps, "count", 3);
    pdal_options_add_str(tailOps, "invert", "false");
    pdal_stage_t* tailStage = pdal_stage_create_tail(tailOps);
    pdal_options_destroy(tailOps);

    RustPipeline pipeline;
    int64_t headIdx = pipeline.addStage(headStage);
    int64_t tailIdx = pipeline.addStage(tailStage);
    pipeline.addDependency(tailIdx, headIdx);

    EXPECT_EQ(pipeline.stageCount(), 2u);

    PointViewPtr output = pipeline.execute(inputView);
    EXPECT_EQ(output->size(), 3u);

    for (PointId i = 0; i < output->size(); ++i)
    {
        double z = output->getFieldAs<double>(Dimension::Id::Z, i);
        EXPECT_EQ(static_cast<int>(z), static_cast<int>(8 + i));
    }
}

TEST(RustPipelineTest, taggedStages)
{
    BOX3D srcBounds(0.0, 0.0, 1.0, 0.0, 0.0, 10.0);
    Options readerOps;
    readerOps.add("bounds", srcBounds);
    readerOps.add("mode", "ramp");
    readerOps.add("count", 10);

    FauxReader reader;
    reader.setOptions(readerOps);

    PointTable table;
    reader.prepare(table);
    PointViewSet readSet = reader.execute(table);
    PointViewPtr inputView = *readSet.begin();

    pdal_options_t* headOps = pdal_options_create();
    pdal_options_add_u64(headOps, "count", 5);
    pdal_options_add_str(headOps, "invert", "false");
    pdal_stage_t* headStage = pdal_stage_create_head(headOps);
    pdal_options_destroy(headOps);

    pdal_options_t* tailOps = pdal_options_create();
    pdal_options_add_u64(tailOps, "count", 3);
    pdal_options_add_str(tailOps, "invert", "false");
    pdal_stage_t* tailStage = pdal_stage_create_tail(tailOps);
    pdal_options_destroy(tailOps);

    RustPipeline pipeline;
    int64_t headIdx = pipeline.addStageTagged(headStage, "head");
    int64_t tailIdx = pipeline.addStageTagged(tailStage, "tail");
    pipeline.addDependency(tailIdx, headIdx);

    EXPECT_EQ(pipeline.findByTag("head"), headIdx);
    EXPECT_EQ(pipeline.findByTag("tail"), tailIdx);
    EXPECT_EQ(pipeline.findByTag("nonexistent"), -1);

    PointViewPtr output = pipeline.execute(inputView);
    EXPECT_EQ(output->size(), 3u);
}

TEST(RustPipelineTest, decimationThroughPipeline)
{
    BOX3D srcBounds(0.0, 0.0, 1.0, 0.0, 0.0, 100.0);
    Options readerOps;
    readerOps.add("bounds", srcBounds);
    readerOps.add("mode", "ramp");
    readerOps.add("count", 100);

    FauxReader reader;
    reader.setOptions(readerOps);

    PointTable table;
    reader.prepare(table);
    PointViewSet readSet = reader.execute(table);
    PointViewPtr inputView = *readSet.begin();
    EXPECT_EQ(inputView->size(), 100u);

    pdal_options_t* ops = pdal_options_create();
    pdal_options_add_f64(ops, "step", 10.0);
    pdal_stage_t* decStage = pdal_stage_create_decimation(ops);
    pdal_options_destroy(ops);

    RustPipeline pipeline;
    pipeline.addStage(decStage);

    PointViewPtr output = pipeline.execute(inputView);
    // step=10 means every 10th point is kept: 10 points from 100
    EXPECT_EQ(output->size(), 10u);
}

TEST(RustPipelineTest, executeCount)
{
    BOX3D srcBounds(0.0, 0.0, 1.0, 0.0, 0.0, 42.0);
    Options readerOps;
    readerOps.add("bounds", srcBounds);
    readerOps.add("mode", "ramp");
    readerOps.add("count", 42);

    FauxReader reader;
    reader.setOptions(readerOps);

    PointTable table;
    reader.prepare(table);
    PointViewSet readSet = reader.execute(table);
    PointViewPtr inputView = *readSet.begin();

    pdal_options_t* ops = pdal_options_create();
    pdal_options_add_u64(ops, "count", 10);
    pdal_options_add_str(ops, "invert", "false");
    pdal_stage_t* headStage = pdal_stage_create_head(ops);
    pdal_options_destroy(ops);

    RustPipeline pipeline;
    pipeline.addStage(headStage);

    uint64_t count = pipeline.executeCount(inputView);
    EXPECT_EQ(count, 10u);
}

TEST(RustPipelineTest, metadataAggregation)
{
    BOX3D srcBounds(0.0, 0.0, 1.0, 0.0, 0.0, 10.0);
    Options readerOps;
    readerOps.add("bounds", srcBounds);
    readerOps.add("mode", "ramp");
    readerOps.add("count", 10);

    FauxReader reader;
    reader.setOptions(readerOps);

    PointTable table;
    reader.prepare(table);
    PointViewSet readSet = reader.execute(table);
    PointViewPtr inputView = *readSet.begin();

    pdal_options_t* headOps = pdal_options_create();
    pdal_options_add_u64(headOps, "count", 5);
    pdal_options_add_str(headOps, "invert", "false");
    pdal_stage_t* headStage = pdal_stage_create_head(headOps);
    pdal_options_destroy(headOps);

    pdal_options_t* tailOps = pdal_options_create();
    pdal_options_add_u64(tailOps, "count", 3);
    pdal_options_add_str(tailOps, "invert", "false");
    pdal_stage_t* tailStage = pdal_stage_create_tail(tailOps);
    pdal_options_destroy(tailOps);

    RustPipeline pipeline;
    pipeline.addStageTagged(headStage, "head");
    pipeline.addStageTagged(tailStage, "tail");

    MetadataNode meta = pipeline.metadata();
    EXPECT_EQ(meta.name(), "pipeline");
    EXPECT_GE(meta.children().size(), 2u);
}

TEST(RustPipelineTest, fauxReaderThroughPipeline)
{
    pdal_options_t* fauxOps = pdal_options_create();
    pdal_options_add_u64(fauxOps, "count", 20);
    pdal_options_add_str(fauxOps, "mode", "ramp");
    pdal_options_add_f64(fauxOps, "minx", 0.0);
    pdal_options_add_f64(fauxOps, "maxx", 19.0);
    pdal_options_add_f64(fauxOps, "miny", 0.0);
    pdal_options_add_f64(fauxOps, "maxy", 19.0);
    pdal_options_add_f64(fauxOps, "minz", 1.0);
    pdal_options_add_f64(fauxOps, "maxz", 20.0);
    pdal_reader_t* fauxReader = pdal_reader_create_faux(fauxOps);
    pdal_options_destroy(fauxOps);
    ASSERT_NE(fauxReader, nullptr);

    RustPipeline pipeline;
    int64_t readerIdx = pipeline.addReader(fauxReader);
    EXPECT_GE(readerIdx, 0);

    PointViewPtr output = pipeline.execute(PointViewPtr());
    ASSERT_NE(output, nullptr);
    EXPECT_EQ(output->size(), 20u);

    EXPECT_TRUE(output->hasDim(Dimension::Id::X));
    EXPECT_TRUE(output->hasDim(Dimension::Id::Y));
    EXPECT_TRUE(output->hasDim(Dimension::Id::Z));
    EXPECT_DOUBLE_EQ(output->getFieldAs<double>(Dimension::Id::X, 19), 19.0);
    EXPECT_DOUBLE_EQ(output->getFieldAs<double>(Dimension::Id::Y, 19), 19.0);
    EXPECT_DOUBLE_EQ(output->getFieldAs<double>(Dimension::Id::Z, 19), 20.0);
}

TEST(RustPipelineTest, fauxReaderFilterWriterPipeline)
{
    pdal_options_t* fauxOps = pdal_options_create();
    pdal_options_add_u64(fauxOps, "count", 30);
    pdal_options_add_str(fauxOps, "mode", "ramp");
    pdal_options_add_f64(fauxOps, "minz", 1.0);
    pdal_options_add_f64(fauxOps, "maxz", 30.0);
    pdal_reader_t* fauxReader = pdal_reader_create_faux(fauxOps);
    pdal_options_destroy(fauxOps);
    ASSERT_NE(fauxReader, nullptr);

    pdal_options_t* headOps = pdal_options_create();
    pdal_options_add_u64(headOps, "count", 10);
    pdal_options_add_str(headOps, "invert", "false");
    pdal_stage_t* headStage = pdal_stage_create_head(headOps);
    pdal_options_destroy(headOps);
    ASSERT_NE(headStage, nullptr);

    pdal_writer_t* nullWriter = pdal_writer_create_null(nullptr);
    ASSERT_NE(nullWriter, nullptr);

    RustPipeline pipeline;
    int64_t readerIdx = pipeline.addReader(fauxReader);
    int64_t filterIdx = pipeline.addStage(headStage);
    int64_t writerIdx = pipeline.addWriter(nullWriter);

    pipeline.addDependency(filterIdx, readerIdx);
    pipeline.addDependency(writerIdx, filterIdx);

    PointViewPtr output = pipeline.execute(PointViewPtr());
    EXPECT_EQ(output, nullptr);

    MetadataNode meta = pipeline.metadata();
    EXPECT_EQ(meta.name(), "pipeline");
    EXPECT_GE(meta.children().size(), 3u);
}

TEST(RustPipelineTest, viewConverterPreservesMeshAndRaster)
{
    PointTable table;
    PointLayoutPtr layout(table.layout());
    layout->registerDim(Dimension::Id::X);
    layout->registerDim(Dimension::Id::Y);
    layout->registerDim(Dimension::Id::Z);

    PointViewPtr input(new PointView(table));
    for (PointId idx = 0; idx < 3; ++idx)
    {
        input->point(idx);
        input->setField(Dimension::Id::X, idx, static_cast<double>(idx));
        input->setField(Dimension::Id::Y, idx, static_cast<double>(idx + 1));
        input->setField(Dimension::Id::Z, idx, static_cast<double>(idx + 2));
    }
    input->createMesh("")->add(0, 1, 2);

    Rasterd* raster =
        input->createRaster("surface", RasterLimits(10, 20, 2, 2, 0.5), -1);
    ASSERT_NE(raster, nullptr);
    raster->at(1, 0) = 42;

    pdal_point_view_t* rustView = rust_view_converter::toRust(input);
    PointViewPtr output = rust_view_converter::fromRust(rustView, input);
    pdal_point_view_destroy(rustView);

    ASSERT_NE(output, nullptr);
    ASSERT_NE(output->mesh(), nullptr);
    EXPECT_EQ(output->mesh()->size(), 1u);
    EXPECT_EQ((*output->mesh())[0].m_a, 0u);
    EXPECT_EQ((*output->mesh())[0].m_b, 1u);
    EXPECT_EQ((*output->mesh())[0].m_c, 2u);

    Rasterd* copied = output->raster("surface");
    ASSERT_NE(copied, nullptr);
    EXPECT_EQ(copied->limits(), raster->limits());
    EXPECT_EQ(copied->initializer(), -1);
    EXPECT_EQ(copied->at(1, 0), 42);
}
