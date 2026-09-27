import unittest
from unittest.mock import patch
from latency_analysis import aggregate, pair, frames


def fixture(offset=0):
    def point(event, at, echo):
        return dict(event=event, mono_ms=at+offset, index=0, echo=echo)
    rows = [point("send_call", 10, False), point("request_start", 0, False),
            point("send_queue_before", 20, False), point("send_queue_after", 50, False),
            point("native_receive", 200, True), point("observed", 250, True),
            point("receive_queue_before", 210, True), point("receive_queue_after", 230, True)]
    rows.append(dict(event="egress", mono_ms=50+offset, index=0, echo=False,
                     fragment=0, fragments=1, allowed=True, native_accepted=True,
                     submit_ms=60+offset, submit_end_ms=61+offset, end_ms=62+offset))
    return rows


class AttributionTests(unittest.TestCase):
    def test_untraced_background_record_has_no_sample_index(self):
        a=[dict(event="summary",role="A",run="test",completed_pairs=7)]
        a.extend(dict(event="roundtrip",index=i,received=True,own_accepted=True,
                      elapsed_ms=100+i) for i in range(7))
        b=[dict(event="summary",role="B",run="test",completed_pairs=7),
           dict(event="background_receive",background=True)]
        with patch("latency_analysis.read",side_effect=[(a,[]),(b,[])]):
            self.assertEqual(aggregate("A","B")["directed_deliveries"],14)
        b[-1]["background"]=False
        with patch("latency_analysis.read",side_effect=[(a,[]),(b,[])]):
            with self.assertRaises(ValueError):aggregate("A","B")

    def responder(self, offset):
        rows=fixture(offset)
        for row in rows:
            row["echo"]=not row["echo"]
        # B receives request, observes it, then queues echo.
        times={"native_receive":0,"observed":10,"receive_queue_before":2,
               "receive_queue_after":8,"send_call":20,"send_queue_before":25,
               "send_queue_after":40}
        for row in rows:
            if row["event"] in times:row["mono_ms"]=times[row["event"]]+offset
        return rows

    def test_arbitrary_device_clock_offset_cancels(self):
        a=fixture()
        first=pair(a,self.responder(1000000),0)
        second=pair(a,self.responder(90000000),0)
        self.assertEqual(first,second)
        self.assertEqual(sum(first["parts"].values()),250)
        self.assertEqual(first["parts"]["delivery_callback_residual_ms"],80)

    def test_refused_frame_cannot_disappear_from_success(self):
        rows=fixture();rows[-1]["native_accepted"]=False
        with self.assertRaises(ValueError):frames(rows,0,False)

    def test_missing_fragment_invalidates_attribution(self):
        rows=fixture();rows[-1]["fragments"]=2
        with self.assertRaises(ValueError):frames(rows,0,False)

    def test_missing_receive_marker_is_not_zero_processing_time(self):
        rows=[r for r in fixture() if r["event"]!="native_receive"]
        with self.assertRaises(ValueError):pair(rows,self.responder(900000),0)


if __name__=="__main__":unittest.main()
